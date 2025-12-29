use crate::{
    card::{Card, CardBase, Edition, Modifier, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card},
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct MadDash {
    pub card_base: CardBase,
}

impl MadDash {
    pub const NAME: &'static str = "Mad Dash";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MadDash {
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
        let mut effects = vec![Effect::DrawCard {
            player_id: self.get_owner_id().clone(),
            count: 1,
        }];
        let cards = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        let prompt = "Mad Dash: Pick a unit to gain Movement +1";
        let picked_card_id = pick_card(self.get_owner_id(), &cards, state, prompt).await;
        effects.push(Effect::add_modifier(
            &picked_card_id,
            Modifier::Movement(1),
            Some(EffectQuery::TurnEnd),
        ));
        effects
    }
}
