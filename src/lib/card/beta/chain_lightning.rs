use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card, pick_option},
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
                mana_cost: 2,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
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

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> Vec<Effect> {
        let caster = state.get_card(caster_id).unwrap();
        let mut effects = vec![];
        let mut last_hit_zone = caster.get_zone();
        let mut first_pick = true;
        loop {
            let units = last_hit_zone
                .get_nearby_units(state, None)
                .iter()
                .map(|c| c.get_id().clone())
                .collect::<Vec<uuid::Uuid>>();
            let picked_card = pick_card(self.get_owner_id(), &units, state, "Chain Lightning: Pick a unit").await;
            effects.push(Effect::TakeDamage {
                card_id: picked_card.clone(),
                from: caster_id.clone(),
                damage: 2,
            });

            if !first_pick {
                effects.push(Effect::RemoveResources {
                    player_id: self.get_owner_id().clone(),
                    mana: 2,
                    thresholds: Thresholds::new(),
                    health: 0,
                });
            }

            let options = vec!["Yes".to_string(), "No".to_string()];
            let pick_option = pick_option(
                self.get_owner_id(),
                &options,
                state,
                "Chain Lightning: Pay 2 to deal an additional 2 damage to another unit?",
            )
            .await;
            if options[pick_option] == "No" {
                break;
            }

            last_hit_zone = state.get_card(&picked_card).unwrap().get_zone();
            first_pick = false;
        }

        effects
    }
}
