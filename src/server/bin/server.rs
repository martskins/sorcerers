use sorcerers::{
    card::{Card, CardType, CardZone},
    game::Game,
    networking::Message,
};
use std::{collections::HashMap, net::SocketAddr};

pub struct Server {
    pub socket: tokio::net::UdpSocket,
    pub active_games: HashMap<uuid::Uuid, Game>,
    pub looking_for_match: Vec<uuid::Uuid>,
    pub player_to_game: HashMap<uuid::Uuid, uuid::Uuid>,
    pub clients: HashMap<uuid::Uuid, SocketAddr>,
}

impl Server {
    pub fn new(socket: tokio::net::UdpSocket) -> Self {
        Self {
            socket,
            active_games: HashMap::new(),
            looking_for_match: vec![],
            player_to_game: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    pub async fn process_message(
        &mut self,
        message: &[u8],
        addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let msg = rmp_serde::from_slice::<Message>(message).unwrap();
        match msg {
            Message::Connect => {
                let player_id = uuid::Uuid::new_v4();
                self.looking_for_match.push(player_id);
                self.clients.insert(player_id, addr);
                self.send_to(&Message::ConnectResponse { player_id }, &addr)
                    .await?;

                match self.find_match() {
                    Some((player1, player2)) => {
                        let game = Game::new(player1, player2);
                        let game_id = game.id.clone();
                        self.player_to_game.insert(player1, game.id);
                        self.player_to_game.insert(player2, game.id);
                        self.active_games.insert(game.id, game);
                        let addr1 = self.clients.get(&player1).unwrap();
                        let addr2 = self.clients.get(&player2).unwrap();
                        self.send_to_many(
                            &Message::MatchCreated { player1, player2 },
                            &[addr1, addr2],
                        )
                        .await?;

                        for player in &[player1, player2] {
                            self.draw_initial_six(player).await?;
                        }
                        self.place_avatars(&game_id)?;
                        self.send_sync(&game_id).await?;
                    }
                    None => {}
                }
            }
            Message::CardPlayed {
                card_id,
                cell_id,
                player_id,
            } => {
                assert!(cell_id >= 1 && cell_id <= 20);
                let game_id = self.player_to_game.get(&player_id).unwrap();
                {
                    let game = self.active_games.get_mut(game_id).unwrap();
                    if let Some(card) = game
                        .cards
                        .iter_mut()
                        .find(|card| card.get_id() == card_id && card.get_owner_id() == &player_id)
                    {
                        card.set_zone(CardZone::Realm(cell_id));
                    }
                }

                self.send_sync(game_id).await?;
            }
            Message::DrawCard {
                card_type,
                player_id,
            } => self.draw_card_for_player(&player_id, card_type).await?,
            _ => {}
        }

        Ok(())
    }

    fn place_avatars(&mut self, game_id: &uuid::Uuid) -> anyhow::Result<()> {
        let game = self.active_games.get_mut(&game_id).unwrap();
        for player_id in &game.players {
            let deck = game.decks.get_mut(&player_id).unwrap();
            let mut avatar_card = Card::Avatar(deck.avatar.clone());
            let cell_id = if game.players[0] == *player_id { 3 } else { 18 };
            avatar_card.set_zone(CardZone::Realm(cell_id));
            game.cards.push(avatar_card);
        }
        Ok(())
    }

    async fn draw_initial_six(&mut self, player_id: &uuid::Uuid) -> anyhow::Result<()> {
        let deck = self
            .active_games
            .get_mut(&self.player_to_game.get(player_id).unwrap())
            .unwrap()
            .decks
            .get_mut(player_id)
            .unwrap();
        deck.shuffle();

        self.draw_card_for_player(&player_id, CardType::Spell)
            .await?;
        self.draw_card_for_player(&player_id, CardType::Spell)
            .await?;
        self.draw_card_for_player(&player_id, CardType::Spell)
            .await?;
        self.draw_card_for_player(&player_id, CardType::Site)
            .await?;
        self.draw_card_for_player(&player_id, CardType::Site)
            .await?;
        self.draw_card_for_player(&player_id, CardType::Site)
            .await?;
        Ok(())
    }

    async fn draw_card_for_player(
        &mut self,
        player_id: &uuid::Uuid,
        card_type: CardType,
    ) -> anyhow::Result<()> {
        let game_id = self.player_to_game.get(&player_id).unwrap();
        {
            let game = self.active_games.get_mut(game_id).unwrap();
            let deck = game.decks.get_mut(&player_id).unwrap();

            let card = match card_type {
                CardType::Site => deck.draw_site().map(|site| Card::Site(site)),
                CardType::Spell => deck.draw_spell().map(|spell| Card::Spell(spell)),
                CardType::Avatar => None,
            };

            if card.is_none() {
                return Ok(());
            }

            let mut card = card.unwrap();
            card.set_zone(CardZone::Hand);
            game.cards.push(card);
        }

        self.send_sync(game_id).await?;
        Ok(())
    }

    async fn send_sync(&self, game_id: &uuid::Uuid) -> anyhow::Result<()> {
        let game = self.active_games.get(&game_id).unwrap();
        for player_id in &game.players {
            let addr = self.clients.get(&player_id).unwrap();
            let player_cards: Vec<Card> = game
                .cards
                .iter()
                .filter(|card| {
                    card.get_owner_id() == player_id
                        || matches!(card.get_zone(), CardZone::Realm(_))
                })
                .cloned()
                .collect();
            let message = Message::Sync {
                cards: player_cards,
            };
            self.send_to(&message, addr).await?;
        }
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

    pub async fn send_to(&self, message: &Message, addr: &SocketAddr) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message)?;
        self.socket.send_to(&bytes, addr).await?;
        Ok(())
    }

    pub async fn send_to_many(
        &self,
        message: &Message,
        addrs: &[&SocketAddr],
    ) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message)?;
        for addr in addrs {
            self.socket.send_to(&bytes, addr).await?;
        }
        Ok(())
    }

    // pub async fn recv(&self) -> anyhow::Result<Message> {
    //     let mut res = [0; 1024];
    //     self.socket.recv(&mut res).await?;
    //     let response: Message = rmp_serde::from_slice(&res).unwrap();
    //     Ok(response)
    // }
}
