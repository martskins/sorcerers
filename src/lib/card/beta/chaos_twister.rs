use rand::seq::IndexedRandom;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ChaosTwister {
    card_base: CardBase,
}

impl ChaosTwister {
    pub const NAME: &'static str = "Chaos Twister";
    pub const DESCRIPTION: &'static str = "Place target minion on the back of your hand, then blow it off from a height of at least one foot. Deal damage equal to its power to each unit atop the site it lands on, including itself.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ChaosTwister {
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
impl Magic for ChaosTwister {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let controller_id = self.get_controller_id(state);
        let Some(target_id) = CardQuery::new()
            .minions()
            // TODO: Is there anything we can do here to prevent us from forgetting to filter by
            // caster region on every single query?
            .in_region(caster.get_region(state).clone())
            .with_source_card(*self.get_id())
            .with_prompt("Choose a minion to fling")
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        let target = state.get_card(&target_id);
        let power = target
            .get_power(state)?
            .ok_or_else(|| anyhow::anyhow!("target has no power"))?;
        let region = target.get_region(state).clone();
        let all_surfaces = Location::all_in_region(Region::Surface);
        let landing_zone = all_surfaces
            .choose(&mut rand::rng())
            .expect("choose to yield one result");
        let mut effects = vec![Effect::TeleportCard {
            player_id: controller_id,
            card_id: target_id,
            to_location: landing_zone.with_region(region),
        }];

        let units = CardQuery::new()
            .units()
            .in_location(landing_zone.clone())
            .id_not(target_id)
            .all(state);
        for unit_id in units {
            effects.push(Effect::TakeDamage {
                card_id: unit_id,
                from: *self.get_id(),
                damage: Damage::basic(power),
            });
        }

        effects.push(Effect::TakeDamage {
            card_id: target_id,
            from: target_id,
            damage: Damage::basic(power),
        });

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (ChaosTwister::NAME, |owner_id: PlayerId| {
    Box::new(ChaosTwister::new(owner_id))
});
