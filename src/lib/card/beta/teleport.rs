use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::{CardQuery, Effect, ZoneQuery},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Teleport {
    pub card_base: CardBase,
}

impl Teleport {
    pub const NAME: &'static str = "Teleport";

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
                rarity: Rarity::Ordinary,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Teleport {
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

    async fn on_cast(&mut self, _state: &State, _caster_id: &uuid::Uuid) -> Vec<Effect> {
        vec![Effect::TeleportUnitToZone {
            player_id: self.get_owner_id().clone(),
            unit_query: CardQuery::OwnedBy {
                owner: self.get_owner_id().clone(),
                prompt: Some("Teleport: Choose an ally to teleport".to_string()),
            },
            zone_query: ZoneQuery::AnySite {
                controlled_by: None,
                prompt: Some("Teleport: Choose site to teleport to".to_string()),
            },
        }]
    }
}
