use crate::{
    card::{Card, CardBase, Edition, Modifier, Plane, Zone},
    effect::Effect,
    game::{Action, BaseAction, PlayerId, Thresholds, pick_action, pick_card},
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
        let actions: Vec<Box<dyn Action>> = vec![Box::new(BaseAction::DrawSite), Box::new(BaseAction::DrawSpell)];
        let action = pick_action(self.get_owner_id(), &actions, state).await;
        let cards = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        let picked_card_id = pick_card(self.get_owner_id(), &cards, state).await;
        let mut effects = action.on_select(Some(self.get_id()), self.get_owner_id(), state).await;
        effects.push(Effect::add_modifier(&picked_card_id, Modifier::Movement(1), Some(1)));
        effects
    }
}
