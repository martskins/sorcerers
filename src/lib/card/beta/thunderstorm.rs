use crate::{
    card::{Aura, AuraBase, Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    query::{CardQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Thunderstorm {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
    pub turns_remaining: u8,
}

impl Thunderstorm {
    pub const NAME: &'static str = "Thunderstorm";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 4,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
            aura_base: AuraBase {},
            turns_remaining: 3,
        }
    }
}

impl Aura for Thunderstorm {}

#[async_trait::async_trait]
impl Card for Thunderstorm {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }

    fn get_valid_play_zones(&self, _state: &State) -> Vec<Zone> {
        Zone::all_intersections()
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(turns) = data.downcast_ref::<u8>() {
            self.turns_remaining = *turns;
        }

        Ok(())
    }

    async fn on_turn_end(&self, state: &State) -> Vec<Effect> {
        if &state.current_player != self.get_controller_id() {
            return vec![];
        }

        let zones = self.get_valid_move_zones(state);
        let affected_zones = self.get_affected_zones(state);
        let units = affected_zones
            .iter()
            .flat_map(|zone| {
                zone.get_units(state, None)
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect::<Vec<uuid::Uuid>>()
            })
            .collect::<Vec<uuid::Uuid>>();
        let mut effects = vec![
            Effect::DealDamageToTarget {
                player_id: self.get_controller_id().clone(),
                query: CardQuery::RandomTarget {
                    id: uuid::Uuid::new_v4(),
                    possible_targets: units,
                },
                from: self.get_id().clone(),
                damage: 3,
            },
            Effect::MoveCard {
                player_id: self.get_controller_id().clone(),
                card_id: self.get_id().clone(),
                from: self.get_zone().clone(),
                to: ZoneQuery::FromOptions {
                    id: uuid::Uuid::new_v4(),
                    options: zones,
                    prompt: Some("Pick a zone to move Thunderstorm to".to_string()),
                },
                tap: false,
                plane: Plane::Surface,
                through_path: None,
            },
        ];

        if self.turns_remaining > 1 {
            effects.push(Effect::SetCardData {
                card_id: self.get_id().clone(),
                data: Box::new(self.turns_remaining - 1),
            });
        } else {
            effects.push(Effect::BuryCard {
                card_id: self.get_id().clone(),
                from: self.get_zone().clone(),
            });
        }

        effects
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }
}
