use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ChainLightning {
    card_base: CardBase,
}

impl ChainLightning {
    pub const NAME: &'static str = "Chain Lightning";
    pub const DESCRIPTION: &'static str = "Deal 2 damage to target unit nearby. Any number of times, you may spend ② to additionally target a new unit nearby the previous one.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "AA"),
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
impl Card for ChainLightning {
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
impl Magic for ChainLightning {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let mut effects = vec![];
        let mut last_hit_location = caster.get_location().clone();
        let mut first_pick = true;
        let mut local_state = state.clone();
        let controller_id = self.get_controller_id(state);
        loop {
            let Some(picked_card_id) = CardQuery::new()
                .units()
                .near_to(&last_hit_location)
                .with_prompt("Pick a unit to deal 2 damage to")
                .with_source_card(*self.get_id())
                .pick(&controller_id, &local_state, false)
                .await?
            else {
                break;
            };

            let effect = Effect::TakeDamage {
                card_id: picked_card_id,
                from: *self.get_id(),
                damage: Damage::basic(2),
            };

            // apply the effect the the local_state to keep track of the updated zones
            // and then apply all effects on that state so that any death triggers are handled and
            // the local_state reflects the game state after applying damage.
            effect.apply(&mut local_state).await?;
            local_state.apply_effects_without_log().await?;

            effects.push(effect);

            if !first_pick {
                let effect = Effect::AdjustMana {
                    player_id: self.get_controller_id(state),
                    mana: -2,
                };
                effect.apply(&mut local_state).await?;
                effects.push(effect);
            }

            force_sync(self.get_controller_id(state), &local_state).await?;

            let additional_hit_cost = Cost::mana_only(2);
            if !additional_hit_cost.can_afford(state, self.get_controller_id(state))? {
                break;
            }

            let options = [BaseOption::Yes, BaseOption::No];
            let option_labels = options.iter().map(|o| o.to_string()).collect::<Vec<_>>();
            let picked_option = pick_option(
                &self.get_controller_id(state),
                &option_labels,
                state,
                "Chain Lightning: Pay 2 to deal an additional 2 damage to another unit?",
                false,
            )
            .await?;
            if options[picked_option] == BaseOption::No {
                break;
            }

            last_hit_location = state.get_card(&picked_card_id).get_location().clone();
            first_pick = false;
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ChainLightning::NAME, |owner_id: PlayerId| {
        Box::new(ChainLightning::new(owner_id))
    });
