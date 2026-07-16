use async_channel::Sender;
use chrono::Datelike;
use sorcerers::{
    booster::BoosterPack,
    card::{self, *},
    collection::CollectedCard,
    deck::{CardNameWithCount, DeckList, precon::PreconDeck},
    game::Game,
    networking::{
        client::Client,
        message::{ClientMessage, DeckChoice, Message, ServerMessage},
    },
    state::{Player, PlayerWithDeck},
    zone::{Location, Zone},
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tokio::{net::tcp::OwnedWriteHalf, sync::Mutex};

use crate::repository::{User, UserRepository};

pub struct Server {
    pub games: HashMap<uuid::Uuid, Sender<ClientMessage>>,
    pub game_players: HashMap<uuid::Uuid, Vec<Player>>,
    pub looking_for_match: Vec<(uuid::Uuid, (Player, DeckChoice))>,
    pub streams: HashMap<uuid::Uuid, Arc<Mutex<OwnedWriteHalf>>>,
    pub addr_to_player: HashMap<std::net::SocketAddr, uuid::Uuid>,
    addr_to_user: HashMap<std::net::SocketAddr, uuid::Uuid>,
    pending_starter_selection: HashMap<std::net::SocketAddr, User>,
    users: UserRepository,
    /// When `true`, seed newly-created games with the local development test board.
    /// Enable with `--test-state` or `SORCERERS_TEST_STATE=1`.
    pub test_state: bool,
}

impl Server {
    pub fn new(test_state: bool, users: UserRepository) -> Self {
        Self {
            looking_for_match: Vec::new(),
            streams: HashMap::new(),
            games: HashMap::new(),
            game_players: HashMap::new(),
            addr_to_player: HashMap::new(),
            addr_to_user: HashMap::new(),
            pending_starter_selection: HashMap::new(),
            users,
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
            Message::ClientMessage(ClientMessage::Register { username, password }) => {
                match self.users.register(username, password).await {
                    Ok(user) => self.begin_authenticated_session(user, stream, addr).await?,
                    Err(error) => {
                        self.send_authentication_failure(error.user_message().to_string(), stream)
                            .await?
                    }
                }
            }
            Message::ClientMessage(ClientMessage::Login { username, password }) => {
                match self.users.verify_login(username, password).await {
                    Ok(user) => self.begin_authenticated_session(user, stream, addr).await?,
                    Err(error) => {
                        self.send_authentication_failure(error.user_message().to_string(), stream)
                            .await?
                    }
                }
            }
            // Authentication must precede all gameplay messages.
            Message::ClientMessage(ClientMessage::Connect) => {
                self.send_authentication_failure(
                    "register or log in before connecting".to_string(),
                    stream,
                )
                .await?;
            }
            Message::ClientMessage(ClientMessage::ChooseStarterDeck { deck }) => {
                let Some(user) = self.pending_starter_selection.remove(addr) else {
                    return Ok(());
                };
                let deck_list = starter_deck_list(deck);
                let cards = collection_from_deck(&deck_list);
                match self
                    .users
                    .complete_starter_selection(user.id, deck, &deck_list, &cards)
                    .await
                {
                    Ok(()) => {
                        self.claim_weekly_boosters(user.id).await?;
                        let collection = self.users.load_collection(user.id).await?;
                        let unopened_booster_packs =
                            self.users.load_unopened_booster_packs(user.id).await?;
                        self.authenticate(
                            user,
                            deck.clone(),
                            vec![deck_list],
                            collection,
                            unopened_booster_packs,
                            stream,
                            addr,
                        )
                        .await?
                    }
                    Err(error) => {
                        self.send_authentication_failure(error.user_message().to_string(), stream)
                            .await?
                    }
                }
            }
            Message::ClientMessage(ClientMessage::OpenBoosterPack { pack_id }) => {
                let Some(&user_id) = self.addr_to_user.get(addr) else {
                    return Ok(());
                };
                if let Some(pack) = self.users.open_booster_pack(user_id, *pack_id).await? {
                    Client::send_to_stream(
                        &ServerMessage::BoosterPackOpened {
                            pack_id: *pack_id,
                            pack,
                        },
                        stream,
                    )
                    .await?;
                }
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
                    .remove(addr)
                    .unwrap_or(uuid::Uuid::nil());
                self.looking_for_match.retain(|(id, _)| id != &player_id);
                self.pending_starter_selection.remove(addr);
                self.addr_to_user.remove(addr);
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

    async fn begin_authenticated_session(
        &mut self,
        user: User,
        stream: Arc<Mutex<OwnedWriteHalf>>,
        addr: &std::net::SocketAddr,
    ) -> anyhow::Result<()> {
        match self.users.selected_starter_deck(user.id).await? {
            Some(deck) => {
                let saved_decks = self.users.load_decks(user.id).await?;
                self.claim_weekly_boosters(user.id).await?;
                let collection = self.users.load_collection(user.id).await?;
                let unopened_booster_packs =
                    self.users.load_unopened_booster_packs(user.id).await?;
                self.authenticate(
                    user,
                    deck,
                    saved_decks,
                    collection,
                    unopened_booster_packs,
                    stream,
                    addr,
                )
                .await
            }
            None => {
                Client::send_to_stream(
                    &ServerMessage::StarterDeckSelection {
                        username: user.username.clone(),
                        available_decks: vec![
                            PreconDeck::BetaFire,
                            PreconDeck::BetaAir,
                            PreconDeck::BetaEarth,
                            PreconDeck::BetaWater,
                        ],
                    },
                    Arc::clone(&stream),
                )
                .await?;
                self.pending_starter_selection.insert(*addr, user);
                Ok(())
            }
        }
    }

    async fn authenticate(
        &mut self,
        user: User,
        starter_deck: PreconDeck,
        saved_decks: Vec<DeckList>,
        collection: Vec<CollectedCard>,
        unopened_booster_packs: Vec<sorcerers::booster::UnopenedBoosterPack>,
        stream: Arc<Mutex<OwnedWriteHalf>>,
        addr: &std::net::SocketAddr,
    ) -> anyhow::Result<()> {
        let user_id = user.id;
        if let Some(previous_player_id) = self.addr_to_player.remove(addr) {
            self.streams.remove(&previous_player_id);
            self.looking_for_match
                .retain(|(player_id, _)| *player_id != previous_player_id);
        }
        let player_id = uuid::Uuid::new_v4();
        Client::send_to_stream(
            &ServerMessage::AuthenticationSuccess {
                player_id,
                username: user.username,
                available_decks: vec![starter_deck],
                saved_decks,
                collection,
                unopened_booster_packs,
            },
            Arc::clone(&stream),
        )
        .await?;
        self.streams.insert(player_id, stream);
        self.addr_to_player.insert(*addr, player_id);
        self.addr_to_user.insert(*addr, user_id);
        Ok(())
    }

    async fn claim_weekly_boosters(&self, user_id: uuid::Uuid) -> anyhow::Result<()> {
        let packs = (0..3).map(|_| BoosterPack::beta()).collect::<Vec<_>>();
        let today = chrono::Utc::now().date_naive();
        let week_start =
            today - chrono::Duration::days(today.weekday().num_days_from_monday().into());

        let _claimed = self
            .users
            .claim_weekly_boosters(user_id, week_start, &packs)
            .await?;
        Ok(())
    }

    async fn send_authentication_failure(
        &self,
        message: String,
        stream: Arc<Mutex<OwnedWriteHalf>>,
    ) -> anyhow::Result<()> {
        Client::send_to_stream(&ServerMessage::AuthenticationFailure { message }, stream).await
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
        game.state.add_card(card);
        let card = card::from_name_and_zone(ApprenticeWizard::NAME, &player_one, Zone::Cemetery);
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            CaptainBaldassare::NAME,
            &player_one,
            Zone::Location(Location::Square(8, Region::Surface)),
        );
        game.state.add_card(card);
        let kite_archer = card::from_name_and_zone(
            KiteArcher::NAME,
            &player_one,
            Zone::Location(Location::Square(8, Region::Surface)),
        );
        game.state.add_card(kite_archer);

        let player_two = game.state.players[1].id;
        let card = card::from_name_and_zone(MountainGiant::NAME, &player_one, Zone::Hand);
        game.state.add_card(card);
        let card = card::from_name_and_zone(ApprenticeWizard::NAME, &player_one, Zone::Hand);
        game.state.add_card(card);
        let card = card::from_name_and_zone(KytheraMechanism::NAME, &player_one, Zone::Hand);
        game.state.add_card(card);
        let card = card::from_name_and_zone(AdeptIllusionist::NAME, &player_one, Zone::Hand);
        game.state.add_card(card);
        let card = card::from_name_and_zone(AdeptIllusionist::NAME, &player_one, Zone::Cemetery);
        game.state.add_card(card);
        let card = card::from_name_and_zone(DwarvenDiggingTeam::NAME, &player_one, Zone::Cemetery);
        game.state.add_card(card);
        let card = card::from_name_and_zone(AdeptIllusionist::NAME, &player_one, Zone::Spellbook);
        game.state.add_card(card);
        let card = card::from_name_and_zone(CallToWar::NAME, &player_one, Zone::Hand);
        game.state.add_card(card);
        let card = card::from_name_and_zone(SummerRiver::NAME, &player_one, Zone::Hand);
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            SummerRiver::NAME,
            &player_one,
            Zone::Location(Location::Square(3, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            SummerRiver::NAME,
            &player_one,
            Zone::Location(Location::Square(9, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            SummerRiver::NAME,
            &player_one,
            Zone::Location(Location::Square(4, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            HumbleVillage::NAME,
            &player_one,
            Zone::Location(Location::Square(6, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            HumbleVillage::NAME,
            &player_one,
            Zone::Location(Location::Square(7, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            LoneTower::NAME,
            &player_one,
            Zone::Location(Location::Square(2, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            AridDesert::NAME,
            &player_one,
            Zone::Location(Location::Square(8, Region::Surface)),
        );
        game.state.add_card(card);

        let card = card::from_name_and_zone(
            AridDesert::NAME,
            &player_two,
            Zone::Location(Location::Square(13, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            AridDesert::NAME,
            &player_two,
            Zone::Location(Location::Square(18, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            KiteArcher::NAME,
            &player_two,
            Zone::Location(Location::Square(3, Region::Surface)),
        );
        game.state.add_card(card);
        let card = card::from_name_and_zone(
            FelbogFrogMen::NAME,
            &player_one,
            Zone::Location(Location::Square(13, Region::Surface)),
        );
        game.state.add_card(card);

        let avatar_id = game.state.get_player_avatar_id(&player_one).unwrap();
        let avatar_card = game.state.get_card_mut(&avatar_id);
        avatar_card.get_unit_base_mut().unwrap().damage = 20;
        avatar_card.get_avatar_base_mut().unwrap().deaths_door = true;

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

fn starter_deck_list(deck: &PreconDeck) -> DeckList {
    let (_, cards) = deck.build(&uuid::Uuid::nil());
    let mut sites = BTreeMap::new();
    let mut spells = BTreeMap::new();
    let mut avatar = String::new();

    for card in cards {
        if card.is_avatar() {
            avatar = card.get_name().to_string();
        } else if matches!(card.get_base().zone, Zone::Atlasbook) {
            *sites.entry(card.get_name().to_string()).or_insert(0) += 1;
        } else {
            *spells.entry(card.get_name().to_string()).or_insert(0) += 1;
        }
    }

    DeckList {
        name: format!("{} Precon", deck.name()),
        avatar,
        sites: card_counts(sites),
        spells: card_counts(spells),
    }
}

fn collection_from_deck(deck: &DeckList) -> Vec<CardNameWithCount> {
    let mut cards = deck.sites.clone();
    cards.extend(deck.spells.clone());
    cards.push(CardNameWithCount {
        count: 1,
        name: deck.avatar.clone(),
        is_foil: false,
    });
    cards
}

fn card_counts(cards: BTreeMap<String, u8>) -> Vec<CardNameWithCount> {
    cards
        .into_iter()
        .map(|(name, count)| CardNameWithCount {
            name,
            count,
            is_foil: false,
        })
        .collect()
}
