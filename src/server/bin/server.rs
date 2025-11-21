use sorcerers::{
    game::{Game, Phase, Resources},
    networking::Message,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

pub struct Server {
    pub socket: Arc<tokio::net::UdpSocket>,
    pub active_games: HashMap<uuid::Uuid, Game>,
    pub looking_for_match: Vec<uuid::Uuid>,
    pub player_to_game: HashMap<uuid::Uuid, uuid::Uuid>,
    pub sockets: HashMap<uuid::Uuid, SocketAddr>,
}

impl Server {
    pub fn new(socket: tokio::net::UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
            active_games: HashMap::new(),
            looking_for_match: vec![],
            player_to_game: HashMap::new(),
            sockets: HashMap::new(),
        }
    }

    pub async fn process_effects(&mut self) -> anyhow::Result<()> {
        for game in self.active_games.values_mut() {
            game.process_effects();
        }
        Ok(())
    }

    pub async fn process_message(&mut self, message: &[u8], addr: SocketAddr) -> anyhow::Result<()> {
        let msg = rmp_serde::from_slice::<Message>(message).unwrap();
        match msg {
            Message::Connect => {
                let player_id = uuid::Uuid::new_v4();
                self.looking_for_match.push(player_id);
                self.sockets.insert(player_id, addr);
                self.send_to_addr(&Message::ConnectResponse { player_id }, &addr)
                    .await?;

                match self.find_match() {
                    Some((player1, player2)) => {
                        let game = self.create_game(&player1, &player2);
                        game.place_avatars()?;
                        game.send_sync().await?;
                        for player in &[player1, player2] {
                            game.draw_initial_six(player).await?;
                        }

                        game.broadcast(&Message::MatchCreated { player1, player2 }).await?;
                    }
                    None => {}
                }
            }
            Message::CardPlayed {
                card_id,
                player_id,
                cell_id,
            } => {
                let game_id = self.player_to_game.get(&player_id).unwrap();
                let game = self.active_games.get_mut(game_id).unwrap();
                game.card_played(&player_id, &card_id, cell_id).await?;
            }
            Message::CardSelected { card_id, player_id } => {
                let game_id = self.player_to_game.get(&player_id).unwrap();
                let game = self.active_games.get_mut(game_id).unwrap();
                game.card_selected(&player_id, &card_id).await?;
            }
            // Message::DrawCard {
            //     card_type,
            //     player_id,
            // } => self.draw_card_for_player(&player_id, card_type).await?,
            _ => {}
        }

        Ok(())
    }

    pub async fn send_to_addr(&self, message: &Message, addr: &SocketAddr) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message)?;
        self.socket.send_to(&bytes, addr).await?;
        Ok(())
    }

    fn create_game(&mut self, player1: &uuid::Uuid, player2: &uuid::Uuid) -> &mut Game {
        let addr1 = self.sockets.remove(player1).unwrap().clone();
        let addr2 = self.sockets.remove(player2).unwrap().clone();
        let mut game = Game::new(player1.clone(), player2.clone(), self.socket.clone(), addr1, addr2);
        game.state.phase = Phase::TurnStartPhase;
        game.state.resources.insert(player1.clone(), Resources::new());
        game.state.resources.insert(player2.clone(), Resources::new());

        let game_id = game.id;
        self.player_to_game.insert(player1.clone(), game.id);
        self.player_to_game.insert(player2.clone(), game.id);
        self.active_games.insert(game.id, game);

        self.active_games.get_mut(&game_id).unwrap()
    }

    fn find_match(&mut self) -> Option<(uuid::Uuid, uuid::Uuid)> {
        if self.looking_for_match.len() >= 2 {
            let player1 = self.looking_for_match.remove(0);
            let player2 = self.looking_for_match.remove(0);
            Some((player1, player2))
        } else {
            None
        }
    }
}
