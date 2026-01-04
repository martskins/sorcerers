use async_channel::Sender;
use sorcerers::{
    game::Game,
    networking::message::{ClientMessage, Message, PreconDeck, ServerMessage, ToMessage},
    state::{Player, PlayerWithDeck},
};
use std::{collections::HashMap, sync::Arc};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::Mutex};

pub struct Server {
    pub games: HashMap<uuid::Uuid, Sender<ClientMessage>>,
    pub looking_for_match: Vec<(uuid::Uuid, (Player, PreconDeck))>,
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
                self.send_to_stream(
                    ServerMessage::ConnectResponse {
                        player_id,
                        available_decks: vec![PreconDeck::BetaFire, PreconDeck::BetaAir],
                    },
                    Arc::clone(&stream),
                )
                .await?;
                self.streams.insert(player_id, stream);
            }
            Message::ClientMessage(ClientMessage::JoinQueue {
                player_id,
                player_name,
                deck,
            }) => {
                let player = Player {
                    id: player_id.clone(),
                    name: player_name.clone(),
                };
                self.looking_for_match.push((player_id.clone(), (player, deck.clone())));
                self.streams.insert(player_id.clone(), stream);

                match self.find_match() {
                    Some((player1, player2)) => {
                        self.create_game(&player1.0, player1.1, &player2.0, player2.1).await?;
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

    pub async fn create_game(
        &mut self,
        player1: &Player,
        deck1: PreconDeck,
        player2: &Player,
        deck2: PreconDeck,
    ) -> anyhow::Result<()> {
        let (server_tx, server_rx) = async_channel::unbounded();
        let (client_tx, client_rx) = async_channel::unbounded::<ClientMessage>();

        let (deck1, cards1) = deck1.build(&player1.id);
        let (deck2, cards2) = deck2.build(&player2.id);

        let stream1 = self.streams.remove(&player1.id).unwrap().clone();
        let stream2 = self.streams.remove(&player2.id).unwrap().clone();

        let players = vec![
            (
                PlayerWithDeck {
                    player: player1.clone(),
                    deck: deck1,
                    cards: cards1,
                },
                stream1,
            ),
            (
                PlayerWithDeck {
                    player: player2.clone(),
                    deck: deck2,
                    cards: cards2,
                },
                stream2,
            ),
        ];
        let mut game = Game::new(players, client_rx, server_tx, server_rx);
        let player_one = game.state.players[0].id.clone();
        let player_two = game.state.players[1].id.clone();
        game.state.cards.push(sorcerers::card::from_name_and_zone(
            "Gothic Tower",
            &player_one,
            sorcerers::card::Zone::Realm(3),
        ));
        // game.state.cards.push(sorcerers::card::from_name_and_zone(
        //     "Gothic Tower",
        //     &player_one,
        //     sorcerers::card::Zone::Realm(13),
        // ));
        // game.state.cards.push(sorcerers::card::from_name_and_zone(
        //     "Mountain Pass",
        //     &player_one,
        //     sorcerers::card::Zone::Realm(8),
        // ));
        game.state.cards.push(sorcerers::card::from_name_and_zone(
            "Gothic Tower",
            &player_two,
            sorcerers::card::Zone::Realm(18),
        ));
        // game.state.cards.push(sorcerers::card::from_name_and_zone(
        //     "Spectral Stalker",
        //     &player_one,
        //     sorcerers::card::Zone::Realm(8),
        // ));
        // game.state.cards.push(sorcerers::card::from_name_and_zone(
        //     "Spire Lich",
        //     &player_one,
        //     sorcerers::card::Zone::Realm(13),
        // ));
        self.games.insert(game.id.clone(), client_tx);
        tokio::spawn(async move {
            game.start().await.unwrap();
        });
        Ok(())
    }

    pub fn find_match(&mut self) -> Option<((Player, PreconDeck), (Player, PreconDeck))> {
        if self.looking_for_match.len() >= 2 {
            let player1 = self.looking_for_match.remove(0);
            let player2 = self.looking_for_match.remove(0);
            Some((player1.1, player2.1))
        } else {
            None
        }
    }
}
