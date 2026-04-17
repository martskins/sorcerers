use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction, yes_or_no},
    state::{CardQuery, State},
};

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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(ally_id) = CardQuery::new()
            .units()
            .controlled_by(&controller_id)
            .with_prompt("Grapple Shot: Pick an ally to shoot the projectile")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let ally_card = state.get_card(&ally_id);
        let ally_zone = ally_card.get_zone();
        let direction = pick_direction(
            &controller_id,
            &CARDINAL_DIRECTIONS,
            state,
            "Grapple Shot: Pick a direction",
        )
        .await?;
        let mut cur_zone = ally_zone.clone();
        let mut hit_unit_id = None;
        loop {
            match cur_zone.zone_in_direction(&direction, 1) {
                Some(Zone::Realm(next_sq)) if (1..=20).contains(&next_sq) => {
                    cur_zone = Zone::Realm(next_sq);
                    let units = cur_zone.get_units(state, None);
                    for unit in units {
                        if unit.is_unit() {
                            hit_unit_id = Some(unit.get_id());
                            break;
                        }
                    }
                    if hit_unit_id.is_some() {
                        break;
                    }
                }
                _ => break,
            }
        }

        if let Some(target_id) = hit_unit_id {
            let mut effects = vec![Effect::MoveCard {
                player_id: controller_id,
                card_id: ally_id,
                from: ally_zone.clone(),
                to: cur_zone.clone().into(),
                tap: false,
                region: ally_card.get_region(state).clone(),
                through_path: None,
            }];
            // 5. Ask if you want to strike the hit unit
            let strike = yes_or_no(&controller_id, state, "Strike the hit unit?")
                .await
                .unwrap_or(false);
            if strike {
                effects.push(Effect::Attack {
                    attacker_id: ally_id,
                    defender_id: *target_id,
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
