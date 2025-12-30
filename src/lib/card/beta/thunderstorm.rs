use crate::{
    card::{AuraBase, Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_zone},
    query::ZoneQuery,
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
                mana_cost: 4,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                controller_id: owner_id.clone(),
            },
            aura_base: AuraBase {},
        }
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

    fn get_valid_play_zones(&self, _state: &State) -> Vec<Zone> {
        Zone::all_intersections()
    }

    async fn on_turn_end(&self, state: &State) -> Vec<Effect> {
        let zones = dbg!(self.get_valid_move_zones(state));
        vec![Effect::MoveCard {
            player_id: self.get_controller_id().clone(),
            card_id: self.get_id().clone(),
            from: self.get_zone().clone(),
            to: ZoneQuery::FromOptions {
                options: zones,
                prompt: Some("Pick a zone to move Thunderstorm to".to_string()),
            },
            tap: false,
            plane: Plane::Surface,
        }]
    }
}
