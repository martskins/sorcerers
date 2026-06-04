use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WhirlingBlades {
    card_base: CardBase,
}

impl WhirlingBlades {
    pub const NAME: &'static str = "Whirling Blades";
    pub const DESCRIPTION: &'static str =
        "An ally may take up to two steps, and then strikes each enemy along their entire path.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "AA"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WhirlingBlades {
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
impl Magic for WhirlingBlades {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(ally_id) = CardQuery::new()
            .units()
            .controlled_by(&controller_id)
            .in_play()
            .with_prompt("Pick an ally")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let ally = state.get_card(&ally_id);
        let from_zone = ally.get_zone().clone();
        let mut destinations = ally.get_zones_within_steps_of(state, 2, &from_zone);
        destinations.retain(|zone| zone.get_site(state).is_some());
        destinations.push(from_zone.clone());
        destinations.sort();
        destinations.dedup();
        let destination = pick_zone(
            &controller_id,
            &destinations,
            state,
            false,
            "Whirling Blades: Pick where the ally moves",
        )
        .await?;

        let mut path_zones = vec![from_zone.clone(), destination.clone()];
        path_zones.sort();
        path_zones.dedup();
        let power = ally.get_power(state)?.unwrap_or_default();
        let mut effects = vec![];
        if destination != from_zone {
            effects.push(Effect::MoveCard {
                player_id: controller_id,
                card_id: ally_id,
                from: from_zone
                    .clone()
                    .into_location()
                    .expect("Whirling Blades ally must be in a location"),
                to: LocationQuery::from_zone(
                    destination.with_region(ally.get_region(state).clone()),
                ),
                tap: false,
                through_path: None,
            });
        }

        for enemy_id in CardQuery::new().units().in_zones(&path_zones).all(state) {
            if state.get_card(&enemy_id).get_controller_id(state) != controller_id {
                effects.push(Effect::TakeDamage {
                    card_id: enemy_id,
                    from: ally_id,
                    damage: Damage::strike(power, false),
                });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WhirlingBlades::NAME, |owner_id: PlayerId| {
        Box::new(WhirlingBlades::new(owner_id))
    });
