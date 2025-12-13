use std::{collections::HashMap, sync::Arc};

use crate::{
    card::{Card, CardInfo, CardType, Zone},
    effect::Effect,
    networking::{
        client::Socket,
        message::{ClientMessage, ServerMessage, ToMessage},
    },
    state::State,
};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

pub type PlayerId = uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerStatus {
    None,
    WaitingForCardDraw {
        player_id: PlayerId,
    },
    WaitingForPlay {
        player_id: PlayerId,
    },
    SelectingSquare {
        player_id: PlayerId,
        valid_squares: Vec<u8>,
    },
    SelectingCard {
        player_id: PlayerId,
        valid_cards: Vec<uuid::Uuid>,
    },
    SelectingAction {
        player_id: PlayerId,
        actions: Vec<String>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Element {
    Fire,
    Air,
    Earth,
    Water,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub fire: u8,
    pub air: u8,
    pub earth: u8,
    pub water: u8,
}

impl Thresholds {
    pub fn new() -> Self {
        Thresholds {
            fire: 0,
            air: 0,
            earth: 0,
            water: 0,
        }
    }

    pub fn parse(s: &str) -> Self {
        let mut thresholds = Thresholds::new();
        for c in s.chars() {
            match c {
                'F' => thresholds.fire += 1,
                'A' => thresholds.air += 1,
                'E' => thresholds.earth += 1,
                'W' => thresholds.water += 1,
                _ => {}
            }
        }
        thresholds
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    pub mana: u8,
    pub health: u8,
    pub thresholds: Thresholds,
}

impl Resources {
    pub fn new() -> Self {
        Resources {
            mana: 0,
            health: 20,
            thresholds: Thresholds::new(),
        }
    }

    pub fn can_afford(&self, card: &Box<dyn Card>, state: &State) -> bool {
        let required_thresholds = card.get_required_thresholds(state);
        let cost = card.get_mana_cost(state);
        self.mana >= cost
            && self.thresholds.fire >= required_thresholds.fire
            && self.thresholds.air >= required_thresholds.air
            && self.thresholds.earth >= required_thresholds.earth
            && self.thresholds.water >= required_thresholds.water
    }
}

pub fn are_adjacent(square1: u8, square2: u8) -> bool {
    let row1 = square1 / 5;
    let col1 = square1 % 5;
    let row2 = square2 / 5;
    let col2 = square2 % 5;

    let row_diff = if row1 > row2 { row1 - row2 } else { row2 - row1 };
    let col_diff = if col1 > col2 { col1 - col2 } else { col2 - col1 };

    (row_diff == 1 && col_diff == 0) || (row_diff == 0 && col_diff == 1)
}

pub fn are_nearby(square1: u8, square2: u8) -> bool {
    let row1 = square1 / 5;
    let col1 = square1 % 5;
    let row2 = square2 / 5;
    let col2 = square2 % 5;

    let row_diff = if row1 > row2 { row1 - row2 } else { row2 - row1 };
    let col_diff = if col1 > col2 { col1 - col2 } else { col2 - col1 };

    (row_diff <= 1 && col_diff <= 1) && !(row_diff == 0 && col_diff == 0)
}

pub fn get_nearby_squares(square: u8) -> Vec<u8> {
    (1..=20).filter(|&s| are_nearby(square, s)).collect()
}

pub fn get_adjacent_squares(square: u8) -> Vec<u8> {
    (1..=20).filter(|&s| are_adjacent(square, s)).collect()
}

pub enum InputStatus {
    None,
    PlayingCard { player_id: PlayerId, card_id: uuid::Uuid },
}

pub struct Game {
    pub id: uuid::Uuid,
    pub input_status: InputStatus,
    pub players: Vec<PlayerId>,
    pub state: State,
    pub addrs: HashMap<PlayerId, Socket>,
    pub socket: Arc<UdpSocket>,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid, socket: Arc<UdpSocket>, addr1: Socket, addr2: Socket) -> Self {
        Game {
            id: uuid::Uuid::new_v4(),
            input_status: InputStatus::None,
            state: State::new(Vec::new(), HashMap::new()),
            players: vec![player1, player2],
            addrs: HashMap::from([(player1, addr1), (player2, addr2)]),
            socket,
        }
    }

    pub async fn process_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        match message {
            ClientMessage::PickSquare { square, .. } => {
                if let InputStatus::PlayingCard { player_id, card_id } = &self.input_status {
                    let effects = vec![
                        Effect::PlayCard {
                            player_id: player_id.clone(),
                            card_id: card_id.clone(),
                            square: *square,
                        },
                        Effect::SetPlayerStatus {
                            status: PlayerStatus::WaitingForPlay {
                                player_id: player_id.clone(),
                            },
                        },
                    ];
                    self.state.effects.extend(effects);
                    self.input_status = InputStatus::None;
                }
            }
            ClientMessage::ClickCard { player_id, card_id, .. } => {
                let resources = self.state.resources.get(player_id).unwrap();
                let card = self.state.cards.iter().find(|c| c.get_id() == card_id).unwrap();
                let can_afford = resources.can_afford(card, &self.state);
                if !can_afford {
                    return Ok(());
                }

                if let Zone::Hand = card.get_zone() {
                    let valid_squares = card.get_valid_play_squares(&self.state);
                    self.input_status = InputStatus::PlayingCard {
                        player_id: player_id.clone(),
                        card_id: card_id.clone(),
                    };
                    let effects = vec![Effect::SetPlayerStatus {
                        status: PlayerStatus::SelectingSquare {
                            player_id: player_id.clone(),
                            valid_squares: valid_squares.clone(),
                        },
                    }];
                    self.state.effects.extend(effects);
                }
            }
            ClientMessage::EndTurn { player_id, .. } => {
                let current_index = self.players.iter().position(|p| p == player_id).unwrap();
                let next_player = self.players.iter().cycle().skip(current_index + 1).next();
                self.state.current_player = next_player.unwrap().clone();
                self.state.player_status = PlayerStatus::WaitingForPlay {
                    player_id: self.state.current_player.clone(),
                };
                self.state.turns += 1;
                let effects = vec![
                    Effect::EndTurn {
                        player_id: player_id.clone(),
                    },
                    Effect::StartTurn {
                        player_id: self.state.current_player.clone(),
                    },
                    Effect::SetPlayerStatus {
                        status: PlayerStatus::WaitingForCardDraw {
                            player_id: self.state.current_player.clone(),
                        },
                    },
                ];
                self.state.effects.extend(effects);
            }
            ClientMessage::DrawCard {
                player_id, card_type, ..
            } => {
                let effects = vec![
                    Effect::DrawCard {
                        player_id: player_id.clone(),
                        card_type: card_type.clone(),
                    },
                    Effect::SetPlayerStatus {
                        status: PlayerStatus::WaitingForPlay {
                            player_id: player_id.clone(),
                        },
                    },
                ];

                self.state.effects.extend(effects);
            }
            _ => {}
        }

        let snapshot = self.state.snapshot();
        let effects: Vec<Effect> = self
            .state
            .cards
            .iter_mut()
            .flat_map(|c| c.handle_message(message, &snapshot))
            .collect();
        self.state.effects.extend(effects);

        self.update().await?;
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        self.process_effects()?;
        self.send_sync().await?;
        Ok(())
    }

    pub async fn send_sync(&self) -> anyhow::Result<()> {
        let msg = ServerMessage::Sync {
            cards: self
                .state
                .cards
                .iter()
                .map(|c| CardInfo {
                    id: c.get_id().clone(),
                    name: c.get_name().to_string(),
                    owner_id: c.get_owner_id().clone(),
                    tapped: c.is_tapped(),
                    edition: c.get_edition().clone(),
                    zone: c.get_zone().clone(),
                    card_type: c.get_card_type().clone(),
                })
                .collect(),
            resources: self.state.resources.clone(),
            player_status: self.state.player_status.clone(),
            current_player: self.state.current_player.clone(),
        };

        self.broadcast(&msg).await?;
        Ok(())
    }

    async fn send_message(&self, message: &ServerMessage, addr: &Socket) -> anyhow::Result<()> {
        match addr {
            Socket::SocketAddr(addr) => {
                let bytes = rmp_serde::to_vec(&message.to_message())?;
                self.socket.send_to(&bytes, addr).await?;
            }
            Socket::Noop => {}
        }

        Ok(())
    }

    pub async fn send_to_player(&self, message: &ServerMessage, player_id: &PlayerId) -> anyhow::Result<()> {
        let addr = self.addrs.get(player_id).unwrap();
        self.send_message(message, addr).await
    }

    pub async fn broadcast(&self, message: &ServerMessage) -> anyhow::Result<()> {
        for addr in self.addrs.values() {
            self.send_message(message, addr).await?;
        }
        Ok(())
    }

    pub fn draw_initial_six(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for player_id in &self.players {
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Site,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Site,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Site,
            });

            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Spell,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Spell,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Spell,
            });
        }

        effects
    }

    pub fn place_avatars(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for (player_id, deck) in &self.state.decks {
            let avatar_id = deck.avatar;
            let mut square = 3;
            if player_id == &self.players[0] {
                square = 18;
            }

            effects.push(Effect::MoveCard {
                card_id: avatar_id,
                to: Zone::Realm(square),
            });
        }
        effects
    }

    pub fn process_effects(&mut self) -> anyhow::Result<()> {
        while !self.state.effects.is_empty() {
            let effect = self.state.effects.remove(0);
            if let Some(effect) = effect {
                effect.apply(&mut self.state)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_are_adjacent() {
        use crate::game::are_adjacent;

        assert!(are_adjacent(1, 2));
        assert!(are_adjacent(3, 2));
        assert!(are_adjacent(3, 4));
        assert!(!are_adjacent(3, 7));
        assert!(!are_adjacent(3, 9));
    }

    #[test]
    fn test_are_nearby() {
        use crate::game::are_nearby;

        assert!(are_nearby(1, 2));
        assert!(are_nearby(3, 2));
        assert!(are_nearby(3, 4));
        assert!(are_nearby(3, 7));
        assert!(are_nearby(3, 9));
    }

    #[test]
    fn test_get_adjacent_squares() {
        use crate::game::get_adjacent_squares;

        let adj = get_adjacent_squares(8);
        assert!(adj.contains(&3));
        assert!(adj.contains(&7));
        assert!(adj.contains(&9));
        assert!(adj.contains(&13));
    }
}
