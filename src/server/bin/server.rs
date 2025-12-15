use sorcerers::{
    card::{self, AridDesert, AskelonPhoenix, ClamorOfHarpies, Flamecaller, MadDash, SacredScarabs, Zone},
    deck::precon,
    game::{Game, PlayerStatus, Resources},
    networking::{
        client::Socket,
        message::{ClientMessage, Message, ServerMessage, ToMessage},
    },
    state::State,
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
        let sockets = HashMap::new();
        let looking_for_match = Vec::new();

        Self {
            socket: Arc::new(socket),
            active_games: HashMap::new(),
            player_to_game: HashMap::new(),
            looking_for_match,
            sockets,
        }
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        for game in self.active_games.values_mut() {
            game.update().await?;
        }

        Ok(())
    }

    pub async fn process_message(&mut self, message: &[u8], addr: SocketAddr) -> anyhow::Result<()> {
        match &rmp_serde::from_slice::<Message>(message).unwrap() {
            Message::ClientMessage(ClientMessage::Connect) => {
                let player_id = uuid::Uuid::new_v4();
                self.looking_for_match.push(player_id);
                self.sockets.insert(player_id, Socket::SocketAddr(addr));
                self.send_to_addr(ServerMessage::ConnectResponse { player_id }, &addr)
                    .await?;

                match self.find_match() {
                    Some((player1, player2)) => {
                        let game = self.create_game(&player2, &player1);
                        game.state.effects.extend(game.place_avatars());
                        game.state.effects.extend(game.draw_initial_six());
                        game.broadcast(&ServerMessage::GameStarted {
                            player1,
                            player2,
                            game_id: game.id.clone(),
                        })
                        .await?;
                        game.process_effects()?;
                        game.send_sync().await?;
                    }
                    None => {}
                }
            }
            Message::ClientMessage(msg) => {
                let game_id = msg.game_id();
                let game = self.active_games.get_mut(&game_id).unwrap();
                game.process_message(&msg).await?;
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn send_to_addr<T: ToMessage>(&self, message: T, addr: &SocketAddr) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message())?;
        self.socket.send_to(&bytes, addr).await?;
        Ok(())
    }

    fn create_game(&mut self, player1: &uuid::Uuid, player2: &uuid::Uuid) -> &mut Game {
        let addr1 = self.sockets.remove(player1).unwrap().clone();
        let addr2 = self.sockets.remove(player2).unwrap().clone();
        let (deck1, cards1) = precon::beta::fire(player1.clone());
        let (deck2, cards2) = precon::beta::fire(player2.clone());
        let mut state = State::new(
            Vec::new().into_iter().chain(cards1).chain(cards2).collect(),
            HashMap::from([(player1.clone(), deck1), (player2.clone(), deck2)]),
        );
        state.current_player = player1.clone();
        state.player_status = PlayerStatus::WaitingForPlay {
            player_id: player1.clone(),
        };
        state.resources.insert(player1.clone(), Resources::new());
        state.resources.insert(player2.clone(), Resources::new());

        state.cards.push(card::from_name_and_zone(
            AridDesert::NAME,
            player2.clone(),
            Zone::Realm(8),
        ));
        state.cards.push(card::from_name_and_zone(
            AridDesert::NAME,
            player2.clone(),
            Zone::Realm(9),
        ));
        state.cards.push(card::from_name_and_zone(
            AridDesert::NAME,
            player1.clone(),
            Zone::Realm(14),
        ));
        state.cards.push(card::from_name_and_zone(
            AridDesert::NAME,
            player1.clone(),
            Zone::Realm(13),
        ));
        state.cards.push(card::from_name_and_zone(
            SacredScarabs::NAME,
            player2.clone(),
            Zone::Realm(8),
        ));
        state.cards.push(card::from_name_and_zone(
            AskelonPhoenix::NAME,
            player1.clone(),
            Zone::Realm(13),
        ));
        state
            .cards
            .push(card::from_name_and_zone(MadDash::NAME, player2.clone(), Zone::Hand));
        state.resources.get_mut(player1).unwrap().mana = 4;
        state.resources.get_mut(player1).unwrap().thresholds.fire = 2;
        state.resources.get_mut(player1).unwrap().thresholds.water = 1;
        state.resources.get_mut(player1).unwrap().thresholds.earth = 1;
        state.resources.get_mut(player2).unwrap().mana = 4;
        state.resources.get_mut(player2).unwrap().thresholds.fire = 2;
        state.resources.get_mut(player2).unwrap().thresholds.water = 1;
        state.resources.get_mut(player2).unwrap().thresholds.earth = 1;

        let mut game = Game::new(player1.clone(), player2.clone(), self.socket.clone(), addr1, addr2);
        game.state = state;
        let game_id = game.id;
        self.player_to_game.insert(player1.clone(), game.id);
        self.player_to_game.insert(player2.clone(), game.id);
        self.active_games.insert(game_id, game);
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
