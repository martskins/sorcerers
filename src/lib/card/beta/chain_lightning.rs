use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::{CardQuery, Effect},
    game::{PlayerId, Thresholds, pick_option, pick_zone},
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
        let mut last_hit_zone = caster.get_zone().clone();
        let mut first_pick = true;
        loop {
            // TODO: This is horribly implemented. We may need a specific ChainLightning effect.
            let zones = last_hit_zone.get_nearby();
            let picked_zone = pick_zone(
                self.get_owner_id(),
                &zones,
                state,
                "Chain Lightning: Pick a nearby zone",
            )
            .await;
            effects.push(Effect::DealDamageToTarget {
                player_id: self.get_owner_id().clone(),
                query: CardQuery::InZone {
                    zone: picked_zone.clone(),
                    owner: None,
                    prompt: Some("Chain Lightning: Pick a unit to deal 2 damage to".to_string()),
                },
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

            last_hit_zone = picked_zone;
            first_pick = false;
        }

        effects
    }
}
