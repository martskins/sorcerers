use std::{collections::HashMap, sync::Arc};

use crate::{
    card::{CardInfo, CardType, Zone},
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
}

pub struct Game {
    pub id: uuid::Uuid,
    pub players: Vec<PlayerId>,
    pub state: State,
    pub addrs: HashMap<PlayerId, Socket>,
    pub socket: Arc<UdpSocket>,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid, socket: Arc<UdpSocket>, addr1: Socket, addr2: Socket) -> Self {
        Game {
            id: uuid::Uuid::new_v4(),
            state: State::new(Vec::new(), HashMap::new()),
            players: vec![player1, player2],
            addrs: HashMap::from([(player1, addr1), (player2, addr2)]),
            socket,
        }
    }

    pub async fn process_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        // match message {
        //     ClientMessage::Connect => unreachable!(),
        //     ClientMessage::PlayCard { card_id, .. } => {}
        //     ClientMessage::PickCard { game_id, card_id, .. } => todo!(),
        //     ClientMessage::PickAction {
        //         game_id, action_idx, ..
        //     } => {
        //     }
        //     ClientMessage::PickSquare { game_id, .. } => todo!(),
        //     ClientMessage::EndTurn { game_id, .. } => todo!(),
        //     ClientMessage::ClickCard { game_id, .. } => {
        //     }
        // }

        let effects: Vec<Effect> = self
            .state
            .cards
            .iter_mut()
            .flat_map(|c| c.handle_message(message, &self.state))
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
                    id: c.get_id(),
                    name: c.get_name().to_string(),
                    owner_id: c.get_owner_id(),
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
