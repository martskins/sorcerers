use crate::{
    card::{Card, CardBase, Edition, Modifier, Plane, Rarity, Zone},
    effect::{Effect, EffectQuery, ModifierCounter},
    game::{PlayerId, Thresholds, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Blaze {
    pub card_base: CardBase,
}

impl Blaze {
    pub const NAME: &'static str = "Blaze";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Blaze {
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

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> Vec<Effect> {
        let units = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        let prompt = "Blaze: Pick an ally";
        let picked_card = pick_card(self.get_owner_id(), &units, state, prompt).await;
        vec![
            Effect::AddModifier {
                card_id: picked_card.clone(),
                counter: ModifierCounter {
                    id: uuid::Uuid::new_v4(),
                    modifier: Modifier::Movement(2),
                    expires_on_effect: Some(EffectQuery::TurnEnd),
                },
            },
            Effect::AddModifier {
                card_id: picked_card,
                counter: ModifierCounter {
                    id: uuid::Uuid::new_v4(),
                    modifier: Modifier::Blaze(2),
                    expires_on_effect: Some(EffectQuery::TurnEnd),
                },
            },
        ]
    }
}
