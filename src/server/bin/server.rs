use async_channel::Sender;
use sorcerers::{
    card::{Zone, from_name_and_zone, *},
    deck::precon,
    game::{Game, Resources},
    networking::message::{ClientMessage, Message, ServerMessage, ToMessage},
    state::State,
};
use std::{collections::HashMap, sync::Arc};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::Mutex};

pub struct Server {
    pub games: HashMap<uuid::Uuid, Sender<ClientMessage>>,
    pub looking_for_match: Vec<uuid::Uuid>,
    pub streams: HashMap<uuid::Uuid, Arc<Mutex<OwnedWriteHalf>>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            looking_for_match: Vec::new(),
            streams: HashMap::new(),
            games: HashMap::new(),
        }
    }

    pub async fn process_message(
        &mut self,
        message: &Message,
        stream: Arc<Mutex<OwnedWriteHalf>>,
    ) -> anyhow::Result<()> {
        match message {
            Message::ClientMessage(ClientMessage::Connect) => {
                let player_id = uuid::Uuid::new_v4();
                self.looking_for_match.push(player_id);
                self.send_to_stream(ServerMessage::ConnectResponse { player_id }, Arc::clone(&stream))
                    .await?;
                self.streams.insert(player_id, stream);

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

    pub async fn send_to_stream<T: ToMessage>(
        &self,
        message: T,
        stream: Arc<Mutex<OwnedWriteHalf>>,
    ) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message())?;
        stream.lock().await.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn create_game(&mut self, player1: &uuid::Uuid, player2: &uuid::Uuid) -> anyhow::Result<()> {
        let (server_tx, server_rx) = async_channel::unbounded();
        let (client_tx, client_rx) = async_channel::unbounded::<ClientMessage>();

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

        state.resources.get_mut(player2).unwrap().thresholds.air = 3;
        state.cards.push(from_name_and_zone(
            Wildfire::NAME,
            player1.clone(),
            Zone::Intersection(vec![1, 2, 6, 7]),
        ));
        state
            .cards
            .push(from_name_and_zone(RaiseDead::NAME, player2.clone(), Zone::Hand));
        state
            .cards
            .push(from_name_and_zone(AridDesert::NAME, player1.clone(), Zone::Realm(3)));
        state
            .cards
            .push(from_name_and_zone(AridDesert::NAME, player1.clone(), Zone::Realm(8)));
        state
            .cards
            .push(from_name_and_zone(AridDesert::NAME, player1.clone(), Zone::Realm(7)));
        state
            .cards
            .push(from_name_and_zone(RaalDromedary::NAME, player1.clone(), Zone::Realm(7)));
        state
            .cards
            .push(from_name_and_zone(LuckyCharm::NAME, player1.clone(), Zone::Hand));
        state
            .cards
            .push(from_name_and_zone(PitVipers::NAME, player1.clone(), Zone::Realm(8)));
        state
            .cards
            .push(from_name_and_zone(LuckyCharm::NAME, player1.clone(), Zone::Realm(8)));
        state
            .cards
            .push(from_name_and_zone(PlanarGate::NAME, player2.clone(), Zone::Realm(13)));
        state
            .cards
            .push(from_name_and_zone(PlanarGate::NAME, player2.clone(), Zone::Realm(12)));
        state.cards.push(from_name_and_zone(
            GrandmasterWizard::NAME,
            player2.clone(),
            Zone::Cemetery,
        ));
        state
            .cards
            .push(from_name_and_zone(PitVipers::NAME, player1.clone(), Zone::Cemetery));
        state
            .cards
            .push(from_name_and_zone(HeadlessHaunt::NAME, player2.clone(), Zone::Cemetery));
        state
            .cards
            .push(from_name_and_zone(PlanarGate::NAME, player2.clone(), Zone::Realm(18)));

        let stream1 = self.streams.remove(player1).unwrap().clone();
        let stream2 = self.streams.remove(player2).unwrap().clone();
        let mut game = Game::new(
            player1.clone(),
            player2.clone(),
            stream1,
            stream2,
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

    pub fn find_match(&mut self) -> Option<(uuid::Uuid, uuid::Uuid)> {
        if self.looking_for_match.len() >= 2 {
            let player1 = self.looking_for_match.remove(0);
            let player2 = self.looking_for_match.remove(0);
            Some((player1, player2))
        } else {
            None
        }
    }
}
