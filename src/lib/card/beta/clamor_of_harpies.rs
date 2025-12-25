use crate::{
    card::{Card, CardBase, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{Action, PlayerId, Thresholds, pick_action, pick_card},
    state::State,
};

#[derive(Debug, Clone, PartialEq)]
enum ClamorOfHarpiesAction {
    Strike,
    DoNotStrike,
}

#[async_trait::async_trait]
impl Action for ClamorOfHarpiesAction {
    fn get_name(&self) -> &str {
        match self {
            ClamorOfHarpiesAction::Strike => "Strike",
            ClamorOfHarpiesAction::DoNotStrike => "Do Not Strike",
        }
    }

    async fn on_select(&self, card_id: Option<&uuid::Uuid>, _player_id: &PlayerId, state: &State) -> Vec<Effect> {
        match self {
            ClamorOfHarpiesAction::Strike => {
                let target_card = state.get_card(card_id.unwrap()).unwrap();
                vec![Effect::take_damage(
                    &target_card.get_id(),
                    card_id.unwrap(),
                    state.get_card(card_id.unwrap()).unwrap().get_power(state).unwrap(),
                )]
            }
            ClamorOfHarpiesAction::DoNotStrike => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClamorOfHarpies {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl ClamorOfHarpies {
    pub const NAME: &'static str = "Clamor of Harpies";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Monster],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 4,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ClamorOfHarpies {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    async fn genesis(&self, state: &State) -> Vec<Effect> {
        let valid_cards: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
            .filter(|c| c.get_power(state).unwrap_or(0) < self.get_power(state).unwrap_or(0))
            .map(|c| c.get_id().clone())
            .collect();
        let prompt = "Clamor of Harpies: Pick a unit to bring here";
        let card_id = pick_card(self.get_owner_id(), &valid_cards, state, prompt).await;
        let card = state.get_card(&card_id).unwrap();
        let actions: Vec<Box<dyn Action>> = vec![
            Box::new(ClamorOfHarpiesAction::Strike),
            Box::new(ClamorOfHarpiesAction::DoNotStrike),
        ];
        let prompt = "Clamor of Harpies: Strike selected unit?";
        let action = pick_action(self.get_owner_id(), &actions, state, prompt).await;
        let mut effects = vec![Effect::MoveCard {
            card_id,
            from: card.get_zone().clone(),
            to: self.get_zone().clone(),
            tap: false,
            plane: self.card_base.plane.clone(),
        }];
        effects.extend(action.on_select(Some(card.get_id()), self.get_owner_id(), state).await);
        effects
    }
}
