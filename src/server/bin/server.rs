use sorcerers::{
    card::{Card, CardType, CardZone},
    game::Game,
    networking::Message,
};
use std::{collections::HashMap, net::SocketAddr};

pub struct Server {
    pub socket: tokio::net::UdpSocket,
    pub active_games: Vec<Game>,
    pub looking_for_match: Vec<uuid::Uuid>,
    pub clients: HashMap<uuid::Uuid, SocketAddr>,
}

impl Server {
    pub fn new(socket: tokio::net::UdpSocket) -> Self {
        Self {
            socket,
            active_games: vec![],
            looking_for_match: vec![],
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
                        self.active_games.push(game);
                        let addr1 = self.clients.get(&player1).unwrap();
                        let addr2 = self.clients.get(&player2).unwrap();
                        self.send_to_many(
                            &Message::MatchCreated { player1, player2 },
                            &[addr1, addr2],
                        )
                        .await?;

                        self.send_to(
                            &Message::Sync {
                                cards: vec![Card {
                                    id: uuid::Uuid::new_v4(),
                                    image: "assets/cards/Red Desert.webp".to_string(),
                                    card_type: CardType::Site,
                                    owner_id: player1,
                                    zone: CardZone::Hand,
                                    name: "Red Desert".to_string(),
                                    mana_cost: 0,
                                    description: None,
                                    tapped: false,
                                }],
                            },
                            addr1,
                        )
                        .await?;
                    }
                    None => {}
                }
            }
            _ => {}
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
