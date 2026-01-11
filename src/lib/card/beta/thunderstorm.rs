use crate::{
    card::{Aura, AuraBase, Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    query::{CardQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Thunderstorm {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
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
                cost: Cost::new(4, "AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
            aura_base: AuraBase {},
        }
    }
}

impl Aura for Thunderstorm {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        let controller_id = self.get_controller_id(state);
        let turns_in_play = state
            .effect_log
            .iter()
            .skip_while(|e| match ***e {
                Effect::PlayCard { ref card_id, .. } if card_id == self.get_id() => false,
                _ => true,
            })
            .filter(|e| match ***e {
                Effect::EndTurn { ref player_id, .. } if player_id == &controller_id => true,
                _ => false,
            })
            .count();

        Ok(turns_in_play >= 3)
    }
}

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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if state.current_player != self.get_controller_id(state) {
            return Ok(vec![]);
        }

        let zones = self.get_valid_move_zones(state)?;
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
        let effects = vec![
            Effect::MoveCard {
                player_id: self.get_controller_id(state).clone(),
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
            Effect::DealDamageToTarget {
                player_id: self.get_controller_id(state).clone(),
                query: CardQuery::RandomTarget {
                    id: uuid::Uuid::new_v4(),
                    possible_targets: units,
                },
                from: self.get_id().clone(),
                damage: 3,
            },
        ];

        Ok(effects)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Thunderstorm::NAME, |owner_id: PlayerId| {
    Box::new(Thunderstorm::new(owner_id))
});
