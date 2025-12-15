use std::{collections::HashMap, sync::Arc};

use crate::{
    card::{Card, CardInfo, CardType, Modifier, Zone},
    effect::Effect,
    networking::{
        client::Socket,
        message::{ClientMessage, ServerMessage, ToMessage},
    },
    state::State,
};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

pub type PlayerId = uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub const CARDINAL_DIRECTIONS: [Direction; 4] = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];

impl Direction {
    pub fn get_name(&self) -> String {
        match self {
            Direction::Up => "Up".to_string(),
            Direction::Down => "Down".to_string(),
            Direction::Left => "Left".to_string(),
            Direction::Right => "Right".to_string(),
        }
    }

    pub fn normalise(&self, board_flipped: bool) -> Direction {
        if board_flipped {
            match self {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
            }
        } else {
            self.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerStatus {
    None,
    WaitingForCardDraw {
        player_id: PlayerId,
    },
    WaitingForPlay {
        player_id: PlayerId,
    },
    SelectingZone {
        player_id: PlayerId,
        valid_zones: Vec<Zone>,
    },
    SelectingCard {
        player_id: PlayerId,
        valid_cards: Vec<uuid::Uuid>,
    },
    SelectingDirection {
        player_id: PlayerId,
        directions: Vec<Direction>,
    },
    SelectingAction {
        player_id: PlayerId,
        actions: Vec<String>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Element {
    Fire,
    Air,
    Earth,
    Water,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub fire: u8,
    pub air: u8,
    pub earth: u8,
    pub water: u8,
}

impl Thresholds {
    pub fn new() -> Self {
        Thresholds {
            fire: 0,
            air: 0,
            earth: 0,
            water: 0,
        }
    }

    pub fn parse(s: &str) -> Self {
        let mut thresholds = Thresholds::new();
        for c in s.chars() {
            match c {
                'F' => thresholds.fire += 1,
                'A' => thresholds.air += 1,
                'E' => thresholds.earth += 1,
                'W' => thresholds.water += 1,
                _ => {}
            }
        }
        thresholds
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    pub mana: u8,
    pub health: u8,
    pub thresholds: Thresholds,
}

impl Resources {
    pub fn new() -> Self {
        Resources {
            mana: 0,
            health: 20,
            thresholds: Thresholds::new(),
        }
    }

    pub fn can_afford(&self, card: &Box<dyn Card>, state: &State) -> bool {
        let required_thresholds = card.get_required_thresholds(state);
        let cost = card.get_mana_cost(state);
        self.mana >= cost
            && self.thresholds.fire >= required_thresholds.fire
            && self.thresholds.air >= required_thresholds.air
            && self.thresholds.earth >= required_thresholds.earth
            && self.thresholds.water >= required_thresholds.water
    }
}

pub fn are_adjacent(square1: &Zone, square2: &Zone) -> bool {
    get_adjacent_zones(square1).contains(&square2)
}

pub fn are_nearby(square1: &Zone, square2: &Zone) -> bool {
    get_nearby_zones(square1).contains(&square2)
}

pub fn get_nearby_zones(zone: &Zone) -> Vec<Zone> {
    let mut adjacent = get_adjacent_zones(zone);
    match zone {
        Zone::Realm(square) => {
            let diagonals = match square % 5 {
                0 => vec![
                    Zone::Realm(square.saturating_add(4)),
                    Zone::Realm(square.saturating_sub(6)),
                ],
                1 => vec![
                    Zone::Realm(square.saturating_sub(4)),
                    Zone::Realm(square.saturating_add(6)),
                ],
                _ => vec![
                    Zone::Realm(square.saturating_sub(4)),
                    Zone::Realm(square.saturating_add(6)),
                    Zone::Realm(square.saturating_add(4)),
                    Zone::Realm(square.saturating_sub(6)),
                ],
            };
            adjacent.extend(diagonals);
            adjacent.retain(|s| s.get_square().unwrap() <= 20);
            adjacent
        }
        _ => vec![],
    }
}

pub fn get_adjacent_zones(zone: &Zone) -> Vec<Zone> {
    match zone {
        &Zone::Realm(square) => {
            let mut adjacent = match square % 5 {
                0 => vec![
                    Zone::Realm(square.saturating_add(5)),
                    Zone::Realm(square.saturating_sub(5)),
                    Zone::Realm(square.saturating_sub(1)),
                    Zone::Realm(square),
                ],
                1 => vec![
                    Zone::Realm(square.saturating_add(5)),
                    Zone::Realm(square.saturating_sub(5)),
                    Zone::Realm(square.saturating_add(1)),
                    Zone::Realm(square),
                ],
                _ => vec![
                    Zone::Realm(square.saturating_add(5)),
                    Zone::Realm(square.saturating_sub(5)),
                    Zone::Realm(square.saturating_add(1)),
                    Zone::Realm(square.saturating_sub(1)),
                    Zone::Realm(square),
                ],
            };
            adjacent.retain(|s| s.get_square().unwrap() <= 20);
            adjacent
        }
        _ => vec![],
    }
}

#[derive(Debug)]
pub enum InputStatus {
    None,
    SelectingAction {
        player_id: PlayerId,
        actions: Vec<Action>,
        card_id: Option<uuid::Uuid>,
    },
    PlayingSpell {
        player_id: PlayerId,
        card_id: uuid::Uuid,
    },
    PlayingCard {
        player_id: PlayerId,
        card_id: uuid::Uuid,
    },
    Attacking {
        player_id: PlayerId,
        attacker_id: uuid::Uuid,
    },
    Moving {
        player_id: PlayerId,
        card_id: uuid::Uuid,
    },
}

#[derive(Debug, Clone)]
pub enum Action {
    Move,
    Attack,
    Defend,
}

impl Action {
    pub fn get_name(&self) -> String {
        match self {
            Action::Move => "Move".to_string(),
            Action::Attack => "Attack".to_string(),
            Action::Defend => "Defend".to_string(),
        }
    }
}

pub struct Game {
    pub id: uuid::Uuid,
    pub input_status: InputStatus,
    pub players: Vec<PlayerId>,
    pub state: State,
    pub addrs: HashMap<PlayerId, Socket>,
    pub socket: Arc<UdpSocket>,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid, socket: Arc<UdpSocket>, addr1: Socket, addr2: Socket) -> Self {
        Game {
            id: uuid::Uuid::new_v4(),
            input_status: InputStatus::None,
            state: State::new(Vec::new(), HashMap::new()),
            players: vec![player1, player2],
            addrs: HashMap::from([(player1, addr1), (player2, addr2)]),
            socket,
        }
    }

    async fn handle_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        match (&self.input_status, message) {
            (InputStatus::Attacking { attacker_id, .. }, ClientMessage::PickCard { card_id, .. }) => {
                let effects = vec![Effect::Attack {
                    attacker_id: attacker_id.clone(),
                    defender_id: *card_id,
                }];
                self.state.effects.extend(effects);
                self.input_status = InputStatus::None;
                self.state.player_status = PlayerStatus::WaitingForPlay {
                    player_id: self.state.current_player.clone(),
                };
            }
            (InputStatus::PlayingCard { player_id, card_id }, ClientMessage::PickSquare { square, .. }) => {
                let effects = vec![
                    Effect::PlayCard {
                        player_id: player_id.clone(),
                        card_id: card_id.clone(),
                        zone: Zone::Realm(*square),
                    },
                    Effect::SetPlayerStatus {
                        status: PlayerStatus::WaitingForPlay {
                            player_id: player_id.clone(),
                        },
                    },
                ];
                self.state.effects.extend(effects);
                self.input_status = InputStatus::None;
            }
            (InputStatus::None, ClientMessage::ClickCard { player_id, card_id, .. }) => {
                let card = self.state.cards.iter().find(|c| c.get_id() == card_id).unwrap();
                if card.get_owner_id() != player_id {
                    return Ok(());
                }

                if !card.is_spell() {
                    return Ok(());
                }

                match (card.is_unit(), card.get_zone()) {
                    (true, Zone::Hand) => {
                        let resources = self.state.resources.get(player_id).unwrap();
                        let can_afford = resources.can_afford(card, &self.state);
                        if !can_afford {
                            return Ok(());
                        }

                        let valid_squares = card.get_valid_play_zones(&self.state);
                        self.input_status = InputStatus::PlayingCard {
                            player_id: player_id.clone(),
                            card_id: card_id.clone(),
                        };
                        let effects = vec![Effect::select_square(player_id, valid_squares)];
                        self.state.effects.extend(effects);
                    }
                    (false, Zone::Hand) => {
                        let resources = self.state.resources.get(player_id).unwrap();
                        let can_afford = resources.can_afford(card, &self.state);
                        if !can_afford {
                            return Ok(());
                        }

                        let spellcasters = self
                            .state
                            .cards
                            .iter()
                            .filter(|c| c.can_cast(&self.state, card))
                            .map(|c| c.get_id().clone())
                            .collect();
                        self.input_status = InputStatus::PlayingSpell {
                            player_id: player_id.clone(),
                            card_id: card_id.clone(),
                        };
                        self.state
                            .effects
                            .push_back(Effect::select_card(player_id, spellcasters));
                    }
                    (true, Zone::Realm(_)) => {
                        if card.is_tapped() || card.has_modifier(&self.state, Modifier::SummoningSickness) {
                            return Ok(());
                        }

                        let actions = vec![Action::Attack.get_name(), Action::Move.get_name()];
                        self.input_status = InputStatus::SelectingAction {
                            player_id: player_id.clone(),
                            actions: vec![Action::Attack, Action::Move],
                            card_id: Some(card_id.clone()),
                        };
                        self.state.effects.push_back(Effect::select_action(player_id, actions));
                    }
                    _ => {}
                }
            }
            (InputStatus::PlayingSpell { player_id, card_id }, ClientMessage::PickCard { card_id: caster, .. }) => {
                let player_id = player_id.clone();
                let card_id = card_id.clone();
                self.input_status = InputStatus::None;

                let zone = self.state.get_card(caster).unwrap().get_zone();
                self.state.effects.push_back(Effect::PlayMagic {
                    player_id: player_id,
                    caster_id: caster.clone(),
                    card_id: card_id,
                    from: zone.clone(),
                });
                self.state.effects.push_back(Effect::wait_for_play(&player_id));
            }
            (
                InputStatus::SelectingAction {
                    player_id,
                    actions,
                    card_id,
                },
                ClientMessage::PickAction { action_idx, .. },
            ) => match actions[*action_idx] {
                Action::Attack => {
                    let card_id = card_id.unwrap();
                    let player_id = player_id.clone();
                    let card = self.state.cards.iter().find(|c| c.get_id() == &card_id).unwrap();
                    let valid_cards = card.get_valid_attack_targets(&self.state);
                    self.input_status = InputStatus::Attacking {
                        player_id: player_id.clone(),
                        attacker_id: card_id.clone(),
                    };
                    self.state
                        .effects
                        .push_back(Effect::select_card(&player_id, valid_cards));
                }
                Action::Move => {
                    let card_id = card_id.unwrap();
                    let player_id = player_id.clone();
                    let card = self.state.cards.iter().find(|c| c.get_id() == &card_id).unwrap();
                    let valid_squares = card.get_valid_move_zones(&self.state);
                    self.input_status = InputStatus::Moving {
                        player_id: player_id.clone(),
                        card_id: card_id.clone(),
                    };
                    self.state
                        .effects
                        .push_back(Effect::select_square(&player_id, valid_squares));
                }
                Action::Defend => {}
            },
            (InputStatus::Moving { player_id, card_id }, ClientMessage::PickSquare { square, .. }) => {
                let card = self.state.cards.iter().find(|c| c.get_id() == card_id).unwrap();
                let effects = vec![
                    Effect::MoveCard {
                        card_id: card_id.clone(),
                        from: card.get_zone().clone(),
                        to: Zone::Realm(*square),
                        tap: true,
                    },
                    Effect::SetPlayerStatus {
                        status: PlayerStatus::WaitingForPlay {
                            player_id: player_id.clone(),
                        },
                    },
                ];
                self.state.effects.extend(effects);
                self.input_status = InputStatus::None;
            }
            (InputStatus::None, ClientMessage::EndTurn { player_id, .. }) => {
                let current_index = self.players.iter().position(|p| p == player_id).unwrap();
                let next_player = self.players.iter().cycle().skip(current_index + 1).next();
                self.state.current_player = next_player.unwrap().clone();
                self.state.player_status = PlayerStatus::WaitingForPlay {
                    player_id: self.state.current_player.clone(),
                };
                self.state.turns += 1;
                let effects = vec![
                    Effect::EndTurn {
                        player_id: player_id.clone(),
                    },
                    Effect::StartTurn {
                        player_id: self.state.current_player.clone(),
                    },
                    Effect::SetPlayerStatus {
                        status: PlayerStatus::WaitingForCardDraw {
                            player_id: self.state.current_player.clone(),
                        },
                    },
                ];
                self.state.effects.extend(effects);
            }
            (
                InputStatus::None,
                ClientMessage::DrawCard {
                    player_id, card_type, ..
                },
            ) => {
                let effects = vec![
                    Effect::DrawCard {
                        player_id: player_id.clone(),
                        card_type: card_type.clone(),
                    },
                    Effect::SetPlayerStatus {
                        status: PlayerStatus::WaitingForPlay {
                            player_id: player_id.clone(),
                        },
                    },
                ];

                self.state.effects.extend(effects);
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn process_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        self.handle_message(message).await?;
        let snapshot = self.state.snapshot();
        let effects: Vec<Effect> = self
            .state
            .cards
            .iter_mut()
            .flat_map(|c| c.handle_message(message, &snapshot))
            .collect();
        self.state.effects.extend(effects);

        self.update().await?;
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        let effects = self.check_damage();
        self.state.effects.extend(effects);
        self.process_effects()?;
        self.send_sync().await?;
        Ok(())
    }

    fn check_damage(&self) -> Vec<Effect> {
        let card_ids: Vec<uuid::Uuid> = self
            .state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
            .filter(|c| {
                let damage = c.get_unit_base().unwrap().damage;
                let toughness = c.get_unit_base().unwrap().toughness;
                damage >= toughness
            })
            .map(|c| c.get_id().clone())
            .collect();

        let mut effects = Vec::new();
        for card_id in card_ids {
            effects.push(Effect::BuryCard { card_id });
        }
        effects
    }

    pub async fn send_sync(&self) -> anyhow::Result<()> {
        let msg = ServerMessage::Sync {
            cards: self
                .state
                .cards
                .iter()
                .map(|c| CardInfo {
                    id: c.get_id().clone(),
                    name: c.get_name().to_string(),
                    owner_id: c.get_owner_id().clone(),
                    tapped: c.is_tapped(),
                    edition: c.get_edition().clone(),
                    zone: c.get_zone().clone(),
                    card_type: c.get_card_type().clone(),
                    summoning_sickness: c.has_modifier(&self.state, Modifier::SummoningSickness),
                })
                .collect(),
            resources: self.state.resources.clone(),
            player_status: self.state.player_status.clone(),
            current_player: self.state.current_player.clone(),
        };

        self.broadcast(&msg).await?;
        Ok(())
    }

    async fn send_message(&self, message: &ServerMessage, addr: &Socket) -> anyhow::Result<()> {
        match addr {
            Socket::SocketAddr(addr) => {
                let bytes = rmp_serde::to_vec(&message.to_message())?;
                self.socket.send_to(&bytes, addr).await?;
            }
            Socket::Noop => {}
        }

        Ok(())
    }

    pub async fn send_to_player(&self, message: &ServerMessage, player_id: &PlayerId) -> anyhow::Result<()> {
        let addr = self.addrs.get(player_id).unwrap();
        self.send_message(message, addr).await
    }

    pub async fn broadcast(&self, message: &ServerMessage) -> anyhow::Result<()> {
        for addr in self.addrs.values() {
            self.send_message(message, addr).await?;
        }
        Ok(())
    }

    pub fn draw_initial_six(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for player_id in &self.players {
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Site,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Site,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Site,
            });

            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Spell,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Spell,
            });
            effects.push(Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Spell,
            });
        }

        effects
    }

    pub fn place_avatars(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for (player_id, deck) in &self.state.decks {
            let avatar_id = deck.avatar;
            let mut square = 3;
            if player_id != &self.state.player_one {
                square = 18;
            }

            effects.push(Effect::MoveCard {
                card_id: avatar_id,
                from: Zone::Spellbook,
                to: Zone::Realm(square),
                tap: false,
            });
        }
        effects
    }

    pub fn process_effects(&mut self) -> anyhow::Result<()> {
        while !self.state.effects.is_empty() {
            let effect = self.state.effects.remove(0);
            if let Some(effect) = effect {
                effect.apply(&mut self.state)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::card::Zone;

    #[test]
    fn test_are_adjacent() {
        use crate::game::are_adjacent;

        assert!(are_adjacent(&Zone::Realm(1), &Zone::Realm(2)));
        assert!(are_adjacent(&Zone::Realm(3), &Zone::Realm(2)));
        assert!(are_adjacent(&Zone::Realm(3), &Zone::Realm(4)));
        assert!(!are_adjacent(&Zone::Realm(3), &Zone::Realm(7)));
        assert!(!are_adjacent(&Zone::Realm(3), &Zone::Realm(9)));
    }

    #[test]
    fn test_are_nearby() {
        use crate::game::are_nearby;

        assert!(are_nearby(&Zone::Realm(1), &Zone::Realm(2)));
        assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(2)));
        assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(4)));
        assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(7)));
        assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(9)));
    }

    #[test]
    fn test_get_adjacent_squares() {
        use crate::game::get_adjacent_zones;

        let adj = get_adjacent_zones(&Zone::Realm(8));
        assert!(adj.contains(&Zone::Realm(3)));
        assert!(adj.contains(&Zone::Realm(7)));
        assert!(adj.contains(&Zone::Realm(9)));
        assert!(adj.contains(&Zone::Realm(13)));
    }
}
