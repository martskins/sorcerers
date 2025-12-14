use crate::{
    card::{Card, CardBase, CardType, Edition, MessageHandler, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, PlayerStatus, Thresholds},
    networking::message::ClientMessage,
    state::State,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Status {
    None,
    MinionPick,
    StrikeDecision,
}

#[derive(Debug, Clone)]
enum Action {
    Strike,
    DoNotStrike,
}

impl Action {
    pub fn get_name(&self) -> &str {
        match self {
            Action::Strike => "Strike",
            Action::DoNotStrike => "Do Not Strike",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClamorOfHarpies {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
    targeted_minion: uuid::Uuid,
    status: Status,
    actions: Vec<Action>,
}

impl ClamorOfHarpies {
    pub const NAME: &'static str = "Clamor of Harpies";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 4,
                required_thresholds: Thresholds::parse("F"),
            },
            status: Status::None,
            targeted_minion: uuid::Uuid::nil(),
            actions: Vec::new(),
        }
    }
}

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

    fn get_card_type(&self) -> crate::card::CardType {
        CardType::Spell
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn genesis(&mut self, state: &State) -> Vec<Effect> {
        self.status = Status::MinionPick;
        let valid_cards = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
            .filter(|c| match c.get_unit_base() {
                Some(ub) => ub.toughness < self.unit_base.power,
                _ => false,
            })
            .map(|c| c.get_id().clone())
            .collect();
        vec![Effect::select_card(self.get_owner_id(), valid_cards)]
    }
}

impl MessageHandler for ClamorOfHarpies {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        match (&self.status, message) {
            (Status::MinionPick, ClientMessage::PickCard { card_id, .. }) => {
                self.targeted_minion = card_id.clone();
                self.status = Status::StrikeDecision;
                self.actions = vec![Action::Strike, Action::DoNotStrike];
                let actions = vec![
                    Action::Strike.get_name().to_string(),
                    Action::DoNotStrike.get_name().to_string(),
                ];
                vec![Effect::select_action(self.get_owner_id(), actions)]
            }
            (Status::StrikeDecision, ClientMessage::PickAction { action_idx, .. }) => {
                let target_minion = state
                    .cards
                    .iter()
                    .find(|c| c.get_id() == &self.targeted_minion)
                    .unwrap();
                let mut effects = vec![
                    Effect::MoveCard {
                        card_id: self.targeted_minion.clone(),
                        from: target_minion.get_zone().clone(),
                        to: self.get_zone(),
                        tap: false,
                    },
                    Effect::wait_for_play(self.get_owner_id()),
                ];

                match self.actions[*action_idx] {
                    Action::Strike => effects.push(Effect::TakeDamage {
                        card_id: self.targeted_minion.clone(),
                        from: self.get_id().clone(),
                        damage: self.unit_base.power,
                    }),
                    Action::DoNotStrike => {}
                }

                self.status = Status::None;
                effects
            }
            _ => vec![],
        }
    }
}
