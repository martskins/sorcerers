use async_channel::Sender;
use sorcerers::{
    card::{self, *},
    deck::precon,
    game::{Game, Resources},
    networking::{
        client::Socket,
        message::{ClientMessage, Message, ServerMessage, ToMessage},
    },
    state::State,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

pub struct Server {
    pub socket: Arc<tokio::net::UdpSocket>,
    pub games: HashMap<uuid::Uuid, Sender<ClientMessage>>,
    pub looking_for_match: Vec<uuid::Uuid>,
    pub sockets: HashMap<uuid::Uuid, Socket>,
}

impl Server {
    pub fn new(socket: tokio::net::UdpSocket) -> Self {
        let sockets = HashMap::new();
        let looking_for_match = Vec::new();

        Self {
            socket: Arc::new(socket),
            looking_for_match,
            sockets,
            games: HashMap::new(),
        }
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
                        self.create_game(&player1, &player2).await?;
                    }
                    None => {}
                }
            }
            Message::ClientMessage(msg) => {
                let game_id = msg.game_id();
                self.games.get_mut(&game_id).unwrap().send(msg.clone()).await?;
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

    async fn create_game(&mut self, player1: &uuid::Uuid, player2: &uuid::Uuid) -> anyhow::Result<()> {
        let (server_tx, server_rx) = async_channel::unbounded();
        let (client_tx, client_rx) = async_channel::unbounded::<ClientMessage>();

        let addr1 = self.sockets.remove(player1).unwrap().clone();
        let addr2 = self.sockets.remove(player2).unwrap().clone();
        let (deck1, cards1) = precon::beta::fire(player1.clone());
        let (deck2, cards2) = precon::beta::air(player2.clone());
        let mut state = State::new(
            Vec::new().into_iter().chain(cards1).chain(cards2).collect(),
            HashMap::from([(player1.clone(), deck1), (player2.clone(), deck2)]),
            server_tx.clone(),
            client_rx.clone(),
        );
        state.current_player = player1.clone();
        state.player_one = player1.clone();
        state.resources.insert(player1.clone(), Resources::new());
        state.resources.insert(player2.clone(), Resources::new());

        state.cards.push(card::from_name_and_zone(
            RedDesert::NAME,
            player2.clone(),
            Zone::Realm(18),
        ));
        state.cards.push(card::from_name_and_zone(
            RedDesert::NAME,
            player2.clone(),
            Zone::Realm(13),
        ));
        state.cards.push(card::from_name_and_zone(
            RedDesert::NAME,
            player2.clone(),
            Zone::Realm(12),
        ));
        state.cards.push(card::from_name_and_zone(
            RedDesert::NAME,
            player1.clone(),
            Zone::Realm(8),
        ));
        state.cards.push(card::from_name_and_zone(
            Vesuvius::NAME,
            player1.clone(),
            Zone::Realm(7),
        ));
        state.cards.push(card::from_name_and_zone(
            SacredScarabs::NAME,
            player2.clone(),
            Zone::Realm(13),
        ));
        state.cards.push(card::from_name_and_zone(
            PitVipers::NAME,
            player1.clone(),
            Zone::Realm(8),
        ));
        state.cards.push(card::from_name_and_zone(
            ColickyDragonettes::NAME,
            player2.clone(),
            Zone::Realm(12),
        ));
        state.cards.push(card::from_name_and_zone(
            RedDesert::NAME,
            player1.clone(),
            Zone::Realm(1),
        ));
        state.cards.push(card::from_name_and_zone(
            RedDesert::NAME,
            player1.clone(),
            Zone::Realm(2),
        ));
        state.cards.push(card::from_name_and_zone(
            WayfaringPilgrim::NAME,
            player1.clone(),
            Zone::Realm(2),
        ));
        state.cards.push(card::from_name_and_zone(
            RedDesert::NAME,
            player1.clone(),
            Zone::Realm(3),
        ));
        state
            .cards
            .push(card::from_name_and_zone(Incinerate::NAME, player1.clone(), Zone::Hand));
        state
            .cards
            .push(card::from_name_and_zone(Firebolts::NAME, player1.clone(), Zone::Hand));
        state
            .cards
            .push(card::from_name_and_zone(MadDash::NAME, player1.clone(), Zone::Hand));
        state.resources.get_mut(player1).unwrap().mana = 15;
        state.resources.get_mut(player1).unwrap().thresholds.fire = 4;
        state.resources.get_mut(player1).unwrap().thresholds.water = 1;
        state.resources.get_mut(player1).unwrap().thresholds.earth = 1;
        state.resources.get_mut(player2).unwrap().mana = 15;
        state.resources.get_mut(player2).unwrap().thresholds.fire = 4;
        state.resources.get_mut(player2).unwrap().thresholds.water = 1;
        state.resources.get_mut(player2).unwrap().thresholds.earth = 1;

        let mut game = Game::new(
            player1.clone(),
            player2.clone(),
            self.socket.clone(),
            addr1,
            addr2,
            client_rx,
            server_tx,
            server_rx,
        );
        game.state = state;
        let game_id = game.id;
        self.games.insert(game_id, client_tx);
        tokio::spawn(async move {
            game.start().await.unwrap();
        });
        Ok(())
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
