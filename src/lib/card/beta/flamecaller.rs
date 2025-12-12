use crate::{
    card::{AvatarBase, Card, CardBase, CardType, Edition, MessageHandler, Zone},
    effect::Effect,
    game::{PlayerId, PlayerStatus},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    PlaySite,
}

#[derive(Debug, Clone)]
enum Action {
    PlaySite,
    DrawSite,
}

impl Action {
    pub fn get_name(&self) -> &str {
        match self {
            Action::PlaySite => "Play Site",
            Action::DrawSite => "Draw Site",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Flamecaller {
    pub card_base: CardBase,
    pub avatar_base: AvatarBase,
    pub targeted_minion: uuid::Uuid,
    status: Status,
    actions: Vec<Action>,
}

impl Flamecaller {
    pub const NAME: &'static str = "Flamecaller";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
            },
            avatar_base: AvatarBase {},
            targeted_minion: uuid::Uuid::nil(),
            status: Status::None,
            actions: Vec::new(),
        }
    }
}

impl Card for Flamecaller {
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

    fn get_id(&self) -> uuid::Uuid {
        self.card_base.id
    }

    fn get_card_type(&self) -> crate::card::CardType {
        CardType::Avatar
    }

    fn genesis(&mut self, state: &State) -> Vec<Effect> {
        vec![]
    }
}

impl MessageHandler for Flamecaller {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        match message {
            ClientMessage::ClickCard { card_id, player_id, .. } if *card_id == self.card_base.id => {
                self.actions = vec![Action::PlaySite, Action::DrawSite];
                vec![Effect::PromptDecision {
                    player_id: player_id.clone(),
                    options: self.actions.iter().map(|a| a.get_name().to_string()).collect(),
                }]
            }
            ClientMessage::PickAction {
                action_idx, player_id, ..
            } if *player_id == self.card_base.owner_id => {
                let action = &self.actions[*action_idx];
                let valid_cards = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone() == Zone::Hand)
                    .filter(|c| c.get_owner_id() == player_id)
                    .map(|c| c.get_id())
                    .collect();
                match action {
                    Action::PlaySite => {
                        self.status = Status::PlaySite;
                        vec![Effect::SetPlayerStatus {
                            status: PlayerStatus::SelectingCard {
                                player_id: player_id.clone(),
                                valid_cards,
                            },
                        }]
                    }
                    Action::DrawSite => vec![Effect::DrawCard {
                        player_id: self.card_base.owner_id.clone(),
                        card_type: CardType::Site,
                    }],
                }
            }
            _ => vec![],
        }
    }
}
