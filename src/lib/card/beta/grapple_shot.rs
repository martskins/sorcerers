use crate::{effect::FightContext, prelude::*};

#[derive(Debug, Clone)]
pub struct GrappleShot {
    card_base: CardBase,
}

impl GrappleShot {
    pub const NAME: &'static str = "Grapple Shot";
    pub const DESCRIPTION: &'static str = "An ally shoots a projectile. If it hits a unit, the ally is dragged to that location, and may strike the hit unit when it arrives.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GrappleShot {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for GrappleShot {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(ally_id) = CardQuery::new()
            .units()
            .controlled_by(&controller_id)
            .with_prompt("Pick an ally to shoot the projectile")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let ally_card = state.get_card(&ally_id);
        let ally_zone = ally_card.get_zone();
        let direction = pick_direction_source(
            &controller_id,
            &CARDINAL_DIRECTIONS,
            state,
            "Grapple Shot: Pick a direction",
            Some(ally_id),
        )
        .await?;
        let mut cur_zone = ally_zone.clone();
        let mut hit_unit_id = None;
        loop {
            match cur_zone.zone_in_direction(&direction, 1) {
                Some(Zone::Location(Location::Square(next_sq, region)))
                    if (1..=20).contains(&next_sq) =>
                {
                    cur_zone = Zone::Location(Location::Square(next_sq, region));
                    let units = CardQuery::new().units().in_zone(&cur_zone).all(state);
                    if let Some(unit_id) = units.first() {
                        hit_unit_id = Some(*unit_id);
                        break;
                    };
                }
                _ => break,
            }
        }

        if let Some(target_id) = hit_unit_id {
            let mut effects = vec![Effect::MoveCard {
                player_id: controller_id,
                card_id: ally_id,
                from: (ally_zone.clone())
                    .into_location()
                    .expect("MoveCard source must be a location"),
                to: LocationQuery::from_zone(
                    (cur_zone.clone()).with_region(ally_card.get_region(state).clone()),
                ),
                tap: false,
                through_path: None,
            }];
            // 5. Ask if you want to strike the hit unit
            let strike = yes_or_no_source(
                &controller_id,
                state,
                "Strike the hit unit?",
                Some(*caster_id),
            )
            .await
            .unwrap_or(false);
            if strike {
                effects.push(Effect::Fight {
                    attacker_id: ally_id,
                    defender_id: target_id,
                    defending_ids: vec![],
                    damage_assignment: None,
                    context: FightContext::FightOnly,
                });
            }
            Ok(effects)
        } else {
            Ok(vec![])
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GrappleShot::NAME, |owner_id: PlayerId| {
    Box::new(GrappleShot::new(owner_id))
});
