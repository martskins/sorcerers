use sorcerers::{
    game::{Game, Phase, Resources},
    networking::{Message, Socket},
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

pub struct Server {
    pub socket: Arc<tokio::net::UdpSocket>,
    pub active_games: HashMap<uuid::Uuid, Game>,
    pub looking_for_match: Vec<uuid::Uuid>,
    pub player_to_game: HashMap<uuid::Uuid, uuid::Uuid>,
    pub sockets: HashMap<uuid::Uuid, Socket>,
}

impl Server {
    pub fn new(socket: tokio::net::UdpSocket) -> Self {
        // let player_id = uuid::Uuid::new_v4();
        // let sockets = HashMap::from([(player_id, Socket::Noop)]);
        // let looking_for_match = vec![player_id];
        let sockets = HashMap::new();
        let looking_for_match = vec![];

        Self {
            socket: Arc::new(socket),
            active_games: HashMap::new(),
            player_to_game: HashMap::new(),
            looking_for_match,
            sockets,
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
        let game_id = msg.get_game_id();
        match msg {
            Message::Connect => {
                let player_id = uuid::Uuid::new_v4();
                self.looking_for_match.push(player_id);
                self.sockets.insert(player_id, Socket::SocketAddr(addr));
                self.send_to_addr(&Message::ConnectResponse { player_id }, &addr)
                    .await?;

                match self.find_match() {
                    Some((player1, player2)) => {
                        let game = self.create_game(&player2, &player1);
                        game.place_avatars()?;
                        for player in &[player1, player2] {
                            game.draw_initial_six(player).await?;
                        }
                        game.send_sync().await?;

                        game.broadcast(&Message::MatchCreated {
                            player1,
                            player2,
                            game_id: game.id.clone(),
                        })
                        .await?;
                    }
                    None => {}
                }
            }
            _ => {
                if !game_id.is_some() {
                    return Ok(());
                }

                let game = self.active_games.get_mut(&game_id.unwrap()).unwrap();
                game.process_message(msg).await?;
            }
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
        game.state.current_player = player1.clone();
        game.state.phase = Phase::WaitingForPlay {
            player_id: player1.clone(),
        };
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
