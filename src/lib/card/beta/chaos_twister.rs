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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        let all_minions = CardQuery::new().minions().all(state);
        if all_minions.is_empty() {
            return Ok(vec![]);
        }

        let target_id = pick_card_with_preview(
            &controller_id,
            &all_minions,
            state,
            "Chaos Twister: Choose a minion to fling",
        )
        .await?;

        let target = state.get_card(&target_id);
        let power = target
            .get_power(state)?
            .ok_or_else(|| anyhow::anyhow!("target has no power"))?;
        let from_zone = target.get_zone().clone();
        let region = target.get_region(state).clone();

        // Move the minion to a random site zone, then deal power damage to all units there.
        // TODO: Does this count as a random output for things like Lucky Charm?
        let landing_zone = ZoneQuery::random(Zone::all_realm())
            .pick(&controller_id, state)
            .await?;

        let mut effects = vec![Effect::MoveCard {
            player_id: controller_id,
            card_id: target_id,
            from: from_zone
                .into_location()
                .expect("Chaos Twister target must be in a location"),
            to: LocationQuery::from_zone(landing_zone.clone().with_region(region)),
            tap: false,
            through_path: None,
        }];

        let units = CardQuery::new()
            .units()
            .in_zone(&landing_zone)
            .id_not(&target_id)
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
