use crate::{
    card::{AvatarBase, Card, CardBase, CardType, Edition, MessageHandler, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    PickAction,
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
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
    status: Status,
    actions: Vec<Action>,
}

impl Flamecaller {
    pub const NAME: &'static str = "Flamecaller";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
            },
            avatar_base: AvatarBase { playing_site: None },
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
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
}

impl MessageHandler for Flamecaller {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        // TODO: Make this the default avatar behavior
        if message.player_id() != &self.card_base.owner_id {
            return vec![];
        }

        match (&self.status, message) {
            (Status::None, ClientMessage::ClickCard { card_id, player_id, .. }) if card_id == self.get_id() => {
                if self.card_base.tapped {
                    return vec![];
                }

                self.actions = vec![Action::PlaySite, Action::DrawSite];
                self.status = Status::PickAction;
                let actions = self.actions.iter().map(|a| a.get_name().to_string()).collect();
                vec![Effect::select_action(player_id, actions)]
            }
            (
                Status::PickAction,
                ClientMessage::PickAction {
                    action_idx, player_id, ..
                },
            ) => {
                let action = &self.actions[*action_idx];
                let valid_cards = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone() == &Zone::Hand)
                    .filter(|c| c.get_owner_id() == player_id)
                    .map(|c| c.get_id().clone())
                    .collect();
                match action {
                    Action::PlaySite => {
                        self.status = Status::PlaySite;
                        vec![Effect::select_card(player_id, valid_cards)]
                    }
                    Action::DrawSite => vec![
                        Effect::DrawCard {
                            player_id: self.card_base.owner_id.clone(),
                            card_type: CardType::Site,
                        },
                        Effect::wait_for_play(&self.get_owner_id()),
                        Effect::tap_card(&self.get_id()),
                    ],
                }
            }
            (Status::PlaySite, ClientMessage::PickCard { player_id, card_id, .. }) => {
                let card = state.get_card(card_id).unwrap();
                let valid_squares = card
                    .get_valid_play_zones(state)
                    .iter()
                    .map(|c| match c {
                        z @ Zone::Realm(_) => z.clone(),
                        _ => panic!("Invalid zone for playing site"),
                    })
                    .collect();

                self.avatar_base.playing_site = Some(card_id.clone());
                vec![Effect::select_square(player_id, valid_squares)]
            }
            (Status::PlaySite, ClientMessage::PickSquare { square, .. }) => {
                let card_id = self.avatar_base.playing_site.clone().unwrap();
                self.avatar_base.playing_site = None;
                self.status = Status::None;
                vec![
                    Effect::tap_card(&self.get_id()),
                    Effect::PlayCard {
                        player_id: self.card_base.owner_id.clone(),
                        card_id: card_id,
                        zone: Zone::Realm(*square),
                    },
                    Effect::wait_for_play(self.get_owner_id()),
                ]
            }

            _ => vec![],
        }
    }
}
