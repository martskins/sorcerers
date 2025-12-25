use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Blink {
    pub card_base: CardBase,
}

impl Blink {
    pub const NAME: &'static str = "Blink";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Blink {
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
        let cards = state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == caster.get_owner_id())
            .filter(|c| c.get_id() != caster_id)
            .map(|c| c.get_id().clone())
            .collect::<Vec<_>>();
        let picked_card = pick_card(self.get_owner_id(), &cards, state, "Pick an ally to teleport").await;

        let zone = state.get_card(&picked_card).unwrap().get_zone();
        let zones = zone.get_nearby();
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, "Pick a zone to teleport to").await;
        vec![
            Effect::TeleportCard {
                card_id: picked_card.clone(),
                from: zone.clone(),
                to: picked_zone.clone(),
            },
            Effect::DrawCard {
                player_id: self.get_owner_id().clone(),
                count: 1,
            },
        ]
    }
}
