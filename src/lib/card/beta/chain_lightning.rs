use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{BaseOption, PlayerId, Thresholds, force_sync, pick_card, pick_option},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ChainLightning {
    pub card_base: CardBase,
}

impl ChainLightning {
    pub const NAME: &'static str = "Chain Lightning";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ChainLightning {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let mut effects = vec![];
        let mut last_hit_zone = caster.get_zone().clone();
        let mut first_pick = true;
        let mut local_state = state.snapshot();
        loop {
            let units_nearby = last_hit_zone
                .get_nearby_units(state, None)
                .iter()
                .map(|c| c.get_id().clone())
                .collect::<Vec<_>>();
            let picked_card = pick_card(
                self.get_owner_id(),
                &units_nearby,
                &local_state,
                "Chain Lightning: Pick a unit to deal 2 damage to",
            )
            .await?;
            let effect = Effect::TakeDamage {
                card_id: picked_card,
                from: self.get_id().clone(),
                damage: 2,
            };

            // apply the effect the the local_state to keep track of the updated zones
            // and then apply all effects on that state so that any death triggers are handled and
            // the local_state reflects the game state after applying damage.
            effect.apply(&mut local_state).await?;
            local_state.apply_effects_without_log().await?;

            effects.push(effect);

            if !first_pick {
                let effect = Effect::RemoveResources {
                    player_id: self.get_controller_id(state).clone(),
                    mana: 2,
                    thresholds: Thresholds::new(),
                };
                effect.apply(&mut local_state).await?;
                effects.push(effect);
            }

            force_sync(self.get_controller_id(state), &local_state).await?;

            let additional_hit_cost = Cost {
                mana: 2,
                ..Default::default()
            };
            if !additional_hit_cost.can_afford(state, self.get_controller_id(state))? {
                break;
            }

            let options = vec![BaseOption::Yes, BaseOption::No];
            let option_labels = options.iter().map(|o| o.to_string()).collect::<Vec<_>>();
            let picked_option = pick_option(
                &self.get_controller_id(state),
                &option_labels,
                state,
                "Chain Lightning: Pay 2 to deal an additional 2 damage to another unit?",
            )
            .await?;
            if options[picked_option] == BaseOption::No {
                break;
            }

            last_hit_zone = state.get_card(&picked_card).get_zone().clone();
            first_pick = false;
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (ChainLightning::NAME, |owner_id: PlayerId| {
    Box::new(ChainLightning::new(owner_id))
});
