use async_channel::Sender;
use sorcerers::{
    card::{self, *},
    deck::precon::ALL_PRECONS,
    game::Game,
    networking::{
        client::Client,
        message::{ClientMessage, DeckChoice, Message, ServerMessage},
    },
    state::{Player, PlayerWithDeck},
    zone::{Location, Zone},
};
use std::{collections::HashMap, sync::Arc};
use tokio::{net::tcp::OwnedWriteHalf, sync::Mutex};

pub struct Server {
    pub games: HashMap<uuid::Uuid, Sender<ClientMessage>>,
    pub game_players: HashMap<uuid::Uuid, Vec<Player>>,
    pub looking_for_match: Vec<(uuid::Uuid, (Player, DeckChoice))>,
    pub streams: HashMap<uuid::Uuid, Arc<Mutex<OwnedWriteHalf>>>,
    pub addr_to_player: HashMap<std::net::SocketAddr, uuid::Uuid>,
    /// When `true` every `Sync` message sent to clients carries a board
    /// evaluation.  Enable with the `--eval` flag or `SORCERERS_DEBUG_EVAL=1`.
    pub debug_eval: bool,
    /// When `true`, seed newly-created games with the local development test board.
    /// Enable with `--test-state` or `SORCERERS_TEST_STATE=1`.
    pub test_state: bool,
}

impl Server {
    pub fn new(debug_eval: bool, test_state: bool) -> Self {
        Self {
            looking_for_match: Vec::new(),
            streams: HashMap::new(),
            games: HashMap::new(),
            game_players: HashMap::new(),
            addr_to_player: HashMap::new(),
            debug_eval,
            test_state,
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
                Client::send_to_stream(
                    &ServerMessage::ConnectResponse {
                        player_id,
                        available_decks: ALL_PRECONS
                            .iter()
                            .map(|(deck, _)| (*deck).clone())
                            .collect(),
                    },
                    Arc::clone(&stream),
                )
                .await?;
                self.streams.insert(player_id, stream);
                self.addr_to_player.insert(*addr, player_id);
            }
            Message::ClientMessage(ClientMessage::JoinQueue {
                player_id,
                player_name,
                deck,
            }) => {
                let Some(&registered_player_id) = self.addr_to_player.get(addr) else {
                    return Ok(());
                };
                if player_id != &registered_player_id {
                    return Ok(());
                }

                let player = Player {
                    id: registered_player_id,
                    name: player_name.clone(),
                };
                self.looking_for_match
                    .push((registered_player_id, (player, deck.clone())));
                self.streams.insert(registered_player_id, stream);

                if let Some((player1, player2)) = self.find_match() {
                    self.create_game(&player1.0, player1.1, &player2.0, player2.1)
                        .await?;
                }
            }
            Message::ClientMessage(ClientMessage::Disconnect) => {
                let player_id = self
                    .addr_to_player
                    .get(addr)
                    .cloned()
                    .unwrap_or(uuid::Uuid::nil());
                self.looking_for_match.retain(|(id, _)| id != &player_id);
                self.streams.retain(|_, s| !Arc::ptr_eq(s, &stream));

                if player_id == uuid::Uuid::nil() {
                    return Ok(());
                }

                let game_id = self
                    .game_players
                    .iter()
                    .find(|(_, players)| players.iter().any(|p| p.id == player_id))
                    .map(|(game_id, _)| *game_id);
                if let Some(game_id) = game_id
                    && let Some(tx) = self.games.get_mut(&game_id)
                {
                    tx.send(ClientMessage::PlayerDisconnected { game_id, player_id })
                        .await?;
                }
            }
            Message::ClientMessage(msg) => {
                let Some(&registered_player_id) = self.addr_to_player.get(addr) else {
                    return Ok(());
                };
                if msg.player_id() != &registered_player_id {
                    return Ok(());
                }

                let game_id = msg.game_id();
                let is_player_in_game = self
                    .game_players
                    .get(&game_id)
                    .is_some_and(|players| players.iter().any(|p| p.id == registered_player_id));
                if !is_player_in_game {
                    return Ok(());
                }

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

    pub async fn create_game(
        &mut self,
        player1: &Player,
        deck1: DeckChoice,
        player2: &Player,
        deck2: DeckChoice,
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
        game.debug_eval = self.debug_eval;
        self.games.insert(game.id, client_tx);
        self.game_players
            .insert(game.id, vec![player1.clone(), player2.clone()]);

        if self.test_state {
            self.setup_test_state(&mut game);
        }
        tokio::spawn(async move {
            game.start().await.expect("game to start");
        });

        Ok(())
    }

    fn setup_test_state(&mut self, game: &mut Game) {
        let player_one = game.state.players[0].id;
        let card = card::from_name_and_zone(AramosMercenaries::NAME, &player_one, Zone::Cemetery);
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(ApprenticeWizard::NAME, &player_one, Zone::Cemetery);
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            CaptainBaldassare::NAME,
            &player_one,
            Zone::Location(Location::Square(8, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);

        let player_two = game.state.players[1].id;
        let card = card::from_name_and_zone(RollingBoulder::NAME, &player_one, Zone::Hand);
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(LightningBolt::NAME, &player_one, Zone::Hand);
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            HumbleVillage::NAME,
            &player_one,
            Zone::Location(Location::Square(3, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            GothicTower::NAME,
            &player_one,
            Zone::Location(Location::Square(9, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            HumbleVillage::NAME,
            &player_one,
            Zone::Location(Location::Square(4, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            HumbleVillage::NAME,
            &player_one,
            Zone::Location(Location::Square(6, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            HumbleVillage::NAME,
            &player_one,
            Zone::Location(Location::Square(7, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            HumbleVillage::NAME,
            &player_one,
            Zone::Location(Location::Square(2, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            AridDesert::NAME,
            &player_one,
            Zone::Location(Location::Square(8, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);

        let kite_archer = card::from_name_and_zone(
            "Kite Archer",
            &player_one,
            Zone::Location(Location::Square(8, Region::Surface)),
        );
        game.state.cards.insert(*kite_archer.get_id(), kite_archer);
        let card = card::from_name_and_zone(
            AridDesert::NAME,
            &player_two,
            Zone::Location(Location::Square(13, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            AridDesert::NAME,
            &player_two,
            Zone::Location(Location::Square(18, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            "Ultimate Horror",
            &player_two,
            Zone::Location(Location::Square(3, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let card = card::from_name_and_zone(
            FelbogFrogMen::NAME,
            &player_one,
            Zone::Location(Location::Square(13, Region::Surface)),
        );
        game.state.cards.insert(*card.get_id(), card);
        let player_mana = game.state.get_player_mana_mut(&player_one);
        *player_mana = 10;
    }

    pub fn find_match(&mut self) -> Option<((Player, DeckChoice), (Player, DeckChoice))> {
        if self.looking_for_match.len() >= 2 {
            let player1 = self.looking_for_match.remove(0);
            let player2 = self.looking_for_match.remove(0);
            Some((player1.1, player2.1))
        } else {
            None
        }
    }
}
