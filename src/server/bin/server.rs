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
    pub game_players: HashMap<uuid::Uuid, Vec<Player>>,
    pub looking_for_match: Vec<(uuid::Uuid, (Player, PreconDeck))>,
    pub streams: HashMap<uuid::Uuid, Arc<Mutex<OwnedWriteHalf>>>,
    pub addr_to_player: HashMap<std::net::SocketAddr, uuid::Uuid>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            looking_for_match: Vec::new(),
            streams: HashMap::new(),
            games: HashMap::new(),
            game_players: HashMap::new(),
            addr_to_player: HashMap::new(),
        }
    }

    pub async fn process_message(
        &mut self,
        message: &Message,
        stream: Arc<Mutex<OwnedWriteHalf>>,
        addr: &std::net::SocketAddr,
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
                self.addr_to_player.insert(addr.clone(), player_id);
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
            Message::ClientMessage(ClientMessage::Disconnect) => {
                let player_id = self.addr_to_player.get(addr).cloned().unwrap_or(uuid::Uuid::nil());
                self.looking_for_match.retain(|(id, _)| id != &player_id);
                self.streams.retain(|_, s| !Arc::ptr_eq(s, &stream));

                if player_id == uuid::Uuid::nil() {
                    return Ok(());
                }

                let game_id = self
                    .game_players
                    .iter()
                    .find(|(_, players)| players.iter().any(|p| p.id == player_id))
                    .map(|(game_id, _)| game_id.clone());
                if let Some(game_id) = game_id {
                    if let Some(tx) = self.games.get_mut(&game_id) {
                        tx.send(ClientMessage::PlayerDisconnected {
                            game_id: game_id.clone(),
                            player_id,
                        })
                        .await?;
                    }
                }
            }
            Message::ClientMessage(msg) => {
                let game_id = msg.game_id();
                self.games
                    .get_mut(&game_id)
                    .ok_or(anyhow::anyhow!("failed to get game by game id"))?
                    .send(msg.clone())
                    .await?;
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

        let stream1 = self
            .streams
            .remove(&player1.id)
            .ok_or(anyhow::anyhow!("failed to get player1 stream"))?
            .clone();
        let stream2 = self
            .streams
            .remove(&player2.id)
            .ok_or(anyhow::anyhow!("failed to get player2 stream"))?
            .clone();

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
        self.games.insert(game.id.clone(), client_tx);
        self.game_players
            .insert(game.id.clone(), vec![player1.clone(), player2.clone()]);

        // // Uncomment this to setup a basic game state for testing
        // let player_one = game.state.players[0].id.clone();
        // let player_two = game.state.players[1].id.clone();
        // game.state.cards.push(from_name_and_zone(
        //     "Chain Lightning",
        //     &player_one,
        //     sorcerers::card::Zone::Hand,
        // ));
        // game.state.cards.push(from_name_and_zone(
        //     "Lone Tower",
        //     &player_one,
        //     sorcerers::card::Zone::Realm(3),
        // ));
        // game.state.cards.push(from_name_and_zone(
        //     "Lone Tower",
        //     &player_one,
        //     sorcerers::card::Zone::Realm(8),
        // ));
        // let kite_archer = from_name_and_zone("Kite Archer", &player_two, sorcerers::card::Zone::Realm(8));
        // let mut lucky_charm = from_name_and_zone("Lucky Charm", &player_two, sorcerers::card::Zone::Realm(1));
        // lucky_charm.get_artifact_base_mut().unwrap().attached_to = Some(kite_archer.get_id().clone());
        // game.state.cards.push(lucky_charm);
        // game.state.cards.push(kite_archer);
        // game.state.cards.push(from_name_and_zone(
        //     "Arid Desert",
        //     &player_two,
        //     sorcerers::card::Zone::Realm(13),
        // ));
        // game.state.cards.push(from_name_and_zone(
        //     "Arid Desert",
        //     &player_two,
        //     sorcerers::card::Zone::Realm(18),
        // ));
        // game.state.cards.push(from_name_and_zone(
        //     "Rimland Nomads",
        //     &player_two,
        //     sorcerers::card::Zone::Realm(13),
        // ));
        // let resources = game
        //     .state
        //     .resources
        //     .entry(player_one)
        //     .or_insert(sorcerers::game::Resources::new());
        // resources.mana = 6;
        // resources.thresholds.air = 3;

        tokio::spawn(async move {
            game.start().await.expect("game to start");
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
