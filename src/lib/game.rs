use crate::{
    card::{Card, CardType, Modifier, Plane, RenderableCard, Zone},
    effect::Effect,
    networking::message::{ClientMessage, ServerMessage, ToMessage},
    query::ZoneQuery,
    state::{Phase, Player, State},
};
use async_channel::{Receiver, Sender};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, iter::Sum, sync::Arc};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::Mutex};

pub type PlayerId = uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl PlayerAction for Direction {
    fn get_name(&self) -> String {
        match self {
            Direction::Up => "Up".to_string(),
            Direction::Down => "Down".to_string(),
            Direction::Left => "Left".to_string(),
            Direction::Right => "Right".to_string(),
            Direction::TopLeft => "Top Left".to_string(),
            Direction::TopRight => "Top Right".to_string(),
            Direction::BottomLeft => "Bottom Left".to_string(),
            Direction::BottomRight => "Bottom Right".to_string(),
        }
    }
}

impl Direction {
    pub fn normalise(&self, board_flipped: bool) -> Direction {
        if board_flipped {
            match self {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
                Direction::TopLeft => Direction::BottomRight,
                Direction::TopRight => Direction::BottomLeft,
                Direction::BottomLeft => Direction::TopRight,
                Direction::BottomRight => Direction::TopLeft,
            }
        } else {
            self.clone()
        }
    }

    pub fn rotate(&self, times: u8) -> Direction {
        let directions = [
            Direction::Up,
            Direction::TopRight,
            Direction::Right,
            Direction::BottomRight,
            Direction::Down,
            Direction::BottomLeft,
            Direction::Left,
            Direction::TopLeft,
        ];
        let idx = directions.iter().position(|d| d == self).unwrap();
        let new_idx = (idx + times as usize) % directions.len();
        directions[new_idx].clone()
    }
}

pub const CARDINAL_DIRECTIONS: [Direction; 4] = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];

pub async fn pick_card_with_preview(
    player_id: &PlayerId,
    card_ids: &[uuid::Uuid],
    state: &State,
    prompt: &str,
) -> uuid::Uuid {
    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            player_id: player_id.clone(),
            cards: card_ids.to_vec(),
            preview: true,
        })
        .await
        .unwrap();

    loop {
        let msg = state.get_receiver().recv().await.unwrap();
        match msg {
            ClientMessage::PickCard { card_id, .. } => break card_id,
            _ => unreachable!(),
        }
    }
}

pub async fn pick_card(player_id: &PlayerId, card_ids: &[uuid::Uuid], state: &State, prompt: &str) -> uuid::Uuid {
    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            player_id: player_id.clone(),
            cards: card_ids.to_vec(),
            preview: false,
        })
        .await
        .unwrap();

    loop {
        let msg = state.get_receiver().recv().await.unwrap();
        match msg {
            ClientMessage::PickCard { card_id, .. } => break card_id,
            _ => unreachable!(),
        }
    }
}

pub async fn pick_action<'a>(
    player_id: &PlayerId,
    actions: &'a [Box<dyn Action>],
    state: &State,
    prompt: &str,
) -> &'a Box<dyn Action> {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.to_string(),
            player_id: player_id.clone(),
            actions: actions.iter().map(|c| c.get_name().to_string()).collect(),
        })
        .await
        .unwrap();

    loop {
        let msg = state.get_receiver().recv().await.unwrap();
        match msg {
            ClientMessage::PickAction { action_idx, .. } => break &actions[action_idx],
            _ => panic!("expected PickAction, got {:?}", msg),
        }
    }
}

pub async fn resume(player_id: &PlayerId, state: &State) {
    state
        .get_sender()
        .send(ServerMessage::Resume {
            player_id: player_id.clone(),
        })
        .await
        .unwrap();
}

pub async fn wait_for_opponent(player_id: &PlayerId, state: &State, prompt: impl AsRef<str>) {
    state
        .get_sender()
        .send(ServerMessage::Wait {
            player_id: player_id.clone(),
            prompt: prompt.as_ref().to_string(),
        })
        .await
        .unwrap();
}

pub async fn pick_option(player_id: &PlayerId, actions: &[String], state: &State, prompt: impl AsRef<str>) -> usize {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.as_ref().to_string(),
            player_id: player_id.clone(),
            actions: actions.to_vec(),
        })
        .await
        .unwrap();

    loop {
        let msg = state.get_receiver().recv().await.unwrap();
        match msg {
            ClientMessage::PickAction { action_idx, .. } => break action_idx,
            _ => panic!("expected PickAction, got {:?}", msg),
        }
    }
}

pub async fn pick_path(player_id: &PlayerId, paths: &[Vec<Zone>], state: &State, prompt: &str) -> Vec<Zone> {
    state
        .get_sender()
        .send(ServerMessage::PickPath {
            prompt: prompt.to_string(),
            player_id: player_id.clone(),
            paths: paths.to_vec(),
        })
        .await
        .unwrap();

    loop {
        let msg = state.get_receiver().recv().await.unwrap();
        match msg {
            ClientMessage::PickPath { path, .. } => break path,
            _ => panic!("expected PickPath, got {:?}", msg),
        }
    }
}

pub async fn pick_zone(player_id: &PlayerId, zones: &[Zone], state: &State, prompt: &str) -> Zone {
    state
        .get_sender()
        .send(ServerMessage::PickZone {
            prompt: prompt.to_string(),
            player_id: player_id.clone(),
            zones: zones.to_vec(),
        })
        .await
        .unwrap();

    loop {
        let msg = state.get_receiver().recv().await.unwrap();
        match msg {
            ClientMessage::PickZone { zone, .. } => break zone,
            _ => panic!("expected PickSquare, got {:?}", msg),
        }
    }
}

pub async fn pick_direction(player_id: &PlayerId, directions: &[Direction], state: &State, prompt: &str) -> Direction {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.to_string(),
            player_id: player_id.clone(),
            actions: directions.iter().map(|c| c.get_name()).collect(),
        })
        .await
        .unwrap();

    let board_flipped = &state.player_one != player_id;
    loop {
        let msg = state.get_receiver().recv().await.unwrap();
        match msg {
            ClientMessage::PickAction { action_idx, .. } => break directions[action_idx].normalise(board_flipped),
            _ => panic!("expected PickAction, got {:?}", msg),
        }
    }
}

pub trait PlayerAction: std::fmt::Debug {
    fn get_name(&self) -> String;
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
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

impl Sum for Thresholds {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut total = Thresholds::new();
        for t in iter {
            total.fire += t.fire;
            total.air += t.air;
            total.earth += t.earth;
            total.water += t.water;
        }
        total
    }
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

    pub fn has_resources(&self, mana: u8, threshold: Thresholds) -> bool {
        self.mana >= mana
            && self.thresholds.fire >= threshold.fire
            && self.thresholds.air >= threshold.air
            && self.thresholds.earth >= threshold.earth
            && self.thresholds.water >= threshold.water
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
        Zone::Intersection(sqs) => {
            // Nearby zones are all adjacent zones of all squares in the intersection, plus all intersections that share at least one square
            let mut nearby = Vec::new();
            for sq in sqs {
                let realm_zone = Zone::Realm(*sq);
                nearby.extend(get_adjacent_zones(&realm_zone));
                // Add diagonals for each square
                let diagonals = match sq % 5 {
                    0 => vec![Zone::Realm(sq.saturating_add(4)), Zone::Realm(sq.saturating_sub(6))],
                    1 => vec![Zone::Realm(sq.saturating_sub(4)), Zone::Realm(sq.saturating_add(6))],
                    _ => vec![
                        Zone::Realm(sq.saturating_sub(4)),
                        Zone::Realm(sq.saturating_add(6)),
                        Zone::Realm(sq.saturating_add(4)),
                        Zone::Realm(sq.saturating_sub(6)),
                    ],
                };
                nearby.extend(diagonals);
            }
            // Add intersections that share at least one square (excluding self)
            for intersection in Zone::all_intersections() {
                if let Zone::Intersection(isqs) = &intersection {
                    if isqs != sqs && isqs.iter().any(|sq| sqs.contains(sq)) {
                        nearby.push(intersection.clone());
                    }
                }
            }
            // Remove duplicates
            nearby.dedup();
            nearby
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
        Zone::Intersection(locs) => {
            let mut locs = locs.clone();
            locs.sort();
            let mut intersections = vec![
                Zone::Intersection(locs.iter().map(|l| l.saturating_add(5)).collect()),
                Zone::Intersection(locs.iter().map(|l| l.saturating_add(1)).collect()),
            ];

            if locs[0] > 1 {
                intersections.push(Zone::Intersection(locs.iter().map(|l| l.saturating_sub(1)).collect()));
            }

            if locs[0] > 5 {
                intersections.push(Zone::Intersection(locs.iter().map(|l| l.saturating_sub(5)).collect()));
            }

            intersections
        }
        _ => vec![],
    }
}

pub trait CloneBoxedAction {
    fn clone_boxed_action(&self) -> Box<dyn Action>;
}

impl<T> CloneBoxedAction for T
where
    T: 'static + Action + Clone,
{
    fn clone_boxed_action(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
pub trait Action: std::fmt::Debug + Send + Sync + CloneBoxedAction {
    fn get_name(&self) -> &str;
    async fn on_select(&self, card_id: Option<&uuid::Uuid>, player_id: &PlayerId, state: &State) -> Vec<Effect>;
}

impl Clone for Box<dyn Action> {
    fn clone(&self) -> Box<dyn Action> {
        self.clone_boxed_action()
    }
}

#[derive(Debug, Clone)]
pub enum InputStatus {
    None,
    ShootingProjectile {
        player_id: PlayerId,
        card_id: uuid::Uuid,
        caster_id: Option<uuid::Uuid>,
        from: Zone,
        direction: Option<Direction>,
        damage: u8,
        piercing: bool,
    },
    SelectingAction {
        player_id: PlayerId,
        actions: Vec<Box<dyn Action>>,
        card_id: Option<uuid::Uuid>,
    },
    PlayingSite {
        player_id: PlayerId,
        site_id: Option<uuid::Uuid>,
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
pub enum BaseAction {
    DrawSite,
    DrawSpell,
    Cancel,
}

#[async_trait::async_trait]
impl Action for BaseAction {
    fn get_name(&self) -> &str {
        match self {
            BaseAction::Cancel => "Cancel",
            BaseAction::DrawSite => "Draw Site",
            BaseAction::DrawSpell => "Draw Spell",
        }
    }

    async fn on_select(&self, _: Option<&uuid::Uuid>, player_id: &PlayerId, _: &State) -> Vec<Effect> {
        match self {
            BaseAction::DrawSite => vec![Effect::DrawSite {
                player_id: player_id.clone(),
                count: 1,
            }],
            BaseAction::DrawSpell => vec![Effect::DrawSpell {
                player_id: player_id.clone(),
                count: 1,
            }],
            BaseAction::Cancel => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum AvatarAction {
    PlaySite,
    DrawSite,
}

#[async_trait::async_trait]
impl Action for AvatarAction {
    fn get_name(&self) -> &str {
        match self {
            AvatarAction::PlaySite => "Play Site",
            AvatarAction::DrawSite => "Draw Site",
        }
    }

    async fn on_select(&self, card_id: Option<&uuid::Uuid>, player_id: &PlayerId, state: &State) -> Vec<Effect> {
        match self {
            AvatarAction::PlaySite => {
                let cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone() == &Zone::Hand)
                    .filter(|c| c.get_owner_id() == player_id)
                    .map(|c| c.get_id().clone())
                    .collect();
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await;
                let picked_card = state.get_card(&picked_card_id).unwrap();
                let zones = picked_card.get_valid_play_zones(state);
                let prompt = "Pick a zone to play the site";
                let zone = pick_zone(player_id, &zones, state, prompt).await;
                vec![
                    Effect::play_card(player_id, &picked_card_id, &zone),
                    Effect::tap_card(card_id.unwrap()),
                ]
            }
            AvatarAction::DrawSite => {
                vec![
                    Effect::DrawSite {
                        player_id: player_id.clone(),
                        count: 1,
                    },
                    Effect::tap_card(card_id.unwrap()),
                ]
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnitAction {
    Move,
    Attack,
    RangedAttack,
    Defend,
    Burrow,
    Submerge,
}

#[async_trait::async_trait]
impl Action for UnitAction {
    fn get_name(&self) -> &str {
        match self {
            UnitAction::Move => "Move",
            UnitAction::Attack => "Attack",
            UnitAction::RangedAttack => "Ranged Attack",
            UnitAction::Defend => "Defend",
            UnitAction::Burrow => "Burrow",
            UnitAction::Submerge => "Submerge",
        }
    }

    async fn on_select(&self, card_id: Option<&uuid::Uuid>, player_id: &PlayerId, state: &State) -> Vec<Effect> {
        match self {
            UnitAction::RangedAttack => {
                let card_id = card_id.unwrap();
                let card = state.get_card(card_id).unwrap();
                let cards = card.get_valid_attack_targets(state, true);
                let prompt = "Pick a unit to attack";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await;
                vec![Effect::RangedStrike {
                    attacker_id: card_id.clone(),
                    defender_id: picked_card_id,
                }]
            }
            UnitAction::Attack => {
                let card_id = card_id.unwrap();
                let card = state.get_card(card_id).unwrap();
                let cards = card.get_valid_attack_targets(state, false);
                let prompt = "Pick a unit to attack";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await;
                vec![Effect::Attack {
                    attacker_id: card_id.clone(),
                    defender_id: picked_card_id,
                }]
            }
            UnitAction::Move => {
                let card_id = card_id.unwrap();
                let card = state.get_card(card_id).unwrap();
                let zones = card.get_valid_move_zones(state);
                let prompt = "Pick a zone to move to";
                let zone = pick_zone(player_id, &zones, state, prompt).await;
                let paths = card.get_valid_move_paths(state, &zone);
                let path = if paths.len() > 1 {
                    let prompt = "Pick a path to move along";
                    pick_path(player_id, &paths, state, prompt).await
                } else {
                    paths.first().unwrap().to_vec()
                };

                let opponent = state.players.iter().find(|p| &p.id != player_id).unwrap();
                let interceptors: Vec<(uuid::Uuid, Zone)> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_controller_id() == &opponent.id)
                    .filter(|c| c.is_unit())
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
                    .filter_map(|c| {
                        for zone in &path {
                            if c.get_valid_move_zones(state).contains(&zone) {
                                return Some((c.get_id().clone(), zone.clone()));
                            }
                        }

                        None
                    })
                    .collect();
                let mut interceptor: Option<(uuid::Uuid, Zone)> = None;
                if !interceptors.is_empty() {
                    let mut options = interceptors
                        .iter()
                        .map(|(id, zone)| {
                            let interceptor_card = state.get_card(id).unwrap();
                            format!(
                                "{} from {} at {}",
                                interceptor_card.get_name(),
                                interceptor_card.get_zone(),
                                zone
                            )
                        })
                        .collect::<Vec<String>>();
                    options.push("Do not intercept".to_string());

                    wait_for_opponent(
                        player_id,
                        state,
                        format!("Wait for opponent to choose whether to intersect"),
                    )
                    .await;

                    let action_idx = pick_option(
                        &opponent.id,
                        &options,
                        state,
                        format!("Intercept {} with...", card.get_name()),
                    )
                    .await;
                    if action_idx < interceptors.len() {
                        interceptor = Some(interceptors[action_idx].clone());
                    }

                    resume(player_id, state).await;
                }

                let mut effects = Vec::new();
                for (idx, zone) in path.iter().enumerate() {
                    if idx + 1 >= path.len() {
                        break;
                    }

                    let to_zone = path[idx + 1].clone();
                    effects.push(Effect::MoveCard {
                        player_id: player_id.clone(),
                        card_id: card_id.clone(),
                        from: zone.clone(),
                        to: ZoneQuery::Specific {
                            id: uuid::Uuid::new_v4(),
                            zone: to_zone.clone(),
                        },
                        tap: true,
                        plane: card.get_base().plane.clone(),
                        through_path: None,
                    });

                    if let Some((interceptor_id, zone)) = &interceptor {
                        if &to_zone != zone {
                            continue;
                        }

                        let interceptor_card = state.get_card(&interceptor_id).unwrap();
                        effects.push(Effect::MoveCard {
                            player_id: opponent.id.clone(),
                            card_id: interceptor_id.clone(),
                            from: interceptor_card.get_zone().clone(),
                            to: ZoneQuery::Specific {
                                id: uuid::Uuid::new_v4(),
                                zone: zone.clone(),
                            },
                            tap: true,
                            plane: card.get_base().plane.clone(),
                            through_path: None,
                        });
                        effects.push(Effect::Attack {
                            attacker_id: interceptor_id.clone(),
                            defender_id: card_id.clone(),
                        });

                        break;
                    }
                }

                effects.reverse();
                effects
            }
            UnitAction::Burrow => {
                let card_id = card_id.unwrap();
                vec![Effect::Burrow {
                    card_id: card_id.clone(),
                }]
            }
            UnitAction::Submerge => {
                let card_id = card_id.unwrap();
                vec![Effect::Submerge {
                    card_id: card_id.clone(),
                }]
            }
            UnitAction::Defend => vec![],
        }
    }
}

pub struct Game {
    pub id: uuid::Uuid,
    pub state: State,
    players: Vec<Player>,
    streams: HashMap<PlayerId, Arc<Mutex<OwnedWriteHalf>>>,
    client_receiver: Receiver<ClientMessage>,
    server_receiver: Receiver<ServerMessage>,
}

impl Game {
    pub fn new(
        player1: Player,
        player2: Player,
        addr1: Arc<Mutex<OwnedWriteHalf>>,
        addr2: Arc<Mutex<OwnedWriteHalf>>,
        receiver: Receiver<ClientMessage>,
        server_sender: Sender<ServerMessage>,
        server_receiver: Receiver<ServerMessage>,
    ) -> Self {
        let game_id = uuid::Uuid::new_v4();
        Game {
            id: game_id.clone(),
            state: State::new(
                game_id,
                vec![player1.clone(), player2.clone()],
                Vec::new(),
                HashMap::new(),
                server_sender.clone(),
                receiver.clone(),
            ),
            players: vec![player1.clone(), player2.clone()],
            streams: HashMap::from([(player1.id, addr1), (player2.id, addr2)]),
            client_receiver: receiver,
            server_receiver,
        }
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.state.effects.extend(self.place_avatars());
        self.state.effects.extend(self.draw_initial_six());

        // Process effects before starting the game so players don't see the initial setup in the event log
        self.process_effects().await?;

        self.broadcast(&ServerMessage::GameStarted {
            player1: self.players[0].id.clone(),
            player2: self.players[1].id.clone(),
            game_id: self.id.clone(),
            cards: self.renderables_from_cards(),
        })
        .await?;
        self.process_effects().await?;
        self.send_sync().await?;

        let streams = self.streams.clone();
        let receiver = self.server_receiver.clone();
        tokio::spawn(async move {
            loop {
                let message = receiver.recv().await.unwrap();
                let stream = streams.get(&message.player_id()).unwrap();
                Self::send(Arc::clone(stream), &message).await.unwrap();
            }
        });

        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        loop {
            tokio::select! {
                Ok(message) = self.client_receiver.recv() => {
                    self.process_message(&message).await?;
                }
                _ = interval.tick() => {
                    self.update().await?;
                }
            }
            let message = self.client_receiver.recv().await?;
            self.process_message(&message).await?;
        }
    }

    fn maybe_unblock_effects(&mut self, message: &ClientMessage) {
        if !self.state.waiting_for_input {
            return;
        }

        match message {
            ClientMessage::DrawCard { .. }
            | ClientMessage::PickAction { .. }
            | ClientMessage::PickDirection { .. }
            | ClientMessage::PickCard { .. }
            | ClientMessage::PickZone { .. } => {
                self.state.waiting_for_input = false;
            }
            _ => {}
        }
    }
    async fn handle_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        self.maybe_unblock_effects(message);
        match message {
            ClientMessage::ClickCard { player_id, card_id, .. } => {
                let card = self.state.cards.iter().find(|c| c.get_id() == card_id).unwrap();
                if card.get_owner_id() != player_id {
                    return Ok(());
                }

                if player_id != &self.state.current_player {
                    return Ok(());
                }

                match (card.get_card_type(), card.get_zone()) {
                    (CardType::Artifact, Zone::Hand) => {
                        let resources = self.state.resources.get(&player_id).unwrap();
                        let can_afford = resources.can_afford(card, &self.state);
                        if !can_afford {
                            return Ok(());
                        }

                        let units = card.get_valid_attach_targets(&self.state);
                        let picked_card_id =
                            pick_card(player_id, &units, &self.state, "Pick a unit to attach the relic to").await;
                        let artifact = self.state.get_card_mut(card_id).unwrap();
                        artifact.get_relic_base_mut().unwrap().attached_to = Some(picked_card_id.clone());
                    }
                    (CardType::Minion, Zone::Hand) | (CardType::Aura, Zone::Hand) => {
                        let resources = self.state.resources.get(&player_id).unwrap();
                        let can_afford = resources.can_afford(card, &self.state);
                        if !can_afford {
                            return Ok(());
                        }

                        let zones = card.get_valid_play_zones(&self.state);
                        let prompt = "Pick a zone to play the card";
                        let zone = pick_zone(player_id, &zones, &self.state, prompt).await;
                        self.state
                            .effects
                            .push_back(Effect::play_card(player_id, card_id, &zone));
                    }
                    (CardType::Magic, Zone::Hand) => {
                        let resources = self.state.resources.get(&player_id).unwrap();
                        let can_afford = resources.can_afford(card, &self.state);
                        if !can_afford {
                            return Ok(());
                        }

                        let spellcasters: Vec<uuid::Uuid> = self
                            .state
                            .cards
                            .iter()
                            .filter(|c| c.can_cast(&self.state, card))
                            .map(|c| c.get_id().clone())
                            .collect();
                        let prompt = "Pick a spellcaster to cast the spell";
                        let caster_id = pick_card(player_id, &spellcasters, &self.state, prompt).await;
                        let caster = self.state.get_card(&caster_id).unwrap();
                        self.state.effects.push_back(Effect::PlayMagic {
                            player_id: player_id.clone(),
                            card_id: card_id.clone(),
                            caster_id,
                            from: caster.get_zone().clone(),
                        });
                    }
                    (_, Zone::Realm(_)) => {
                        let unit_disabled =
                            card.is_tapped() || card.has_modifier(&self.state, &Modifier::SummoningSickness);
                        if card.is_unit() && unit_disabled {
                            return Ok(());
                        }

                        let mut actions = card.get_actions(&self.state);
                        if actions.is_empty() {
                            return Ok(());
                        }

                        actions.push(Box::new(BaseAction::Cancel));
                        let prompt = format!("{}: Pick action", card.get_name());
                        let action = pick_action(player_id, &actions, &self.state, &prompt).await;
                        let effects = action.on_select(Some(card.get_id()), player_id, &self.state).await;
                        self.state.effects.extend(effects);
                    }
                    _ => {}
                }
            }
            ClientMessage::EndTurn { player_id, .. } => {
                self.state.effects.push_back(Effect::PreEndTurn {
                    player_id: player_id.clone(),
                });
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn process_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        self.handle_message(message).await?;
        self.update().await?;
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        self.process_effects().await?;

        if let Phase::PreEndTurn { player_id } = self.state.phase {
            // If the current player is not the one ending their turn, it means we've already
            // actioned the pre-end turn changes, so no action is needed.
            if self.state.current_player == player_id && !self.state.waiting_for_input {
                let current_index = self
                    .players
                    .iter()
                    .position(|p| p.id == self.state.current_player)
                    .unwrap();
                let current_player = self.state.current_player.clone();
                let next_player = self.players.iter().cycle().skip(current_index + 1).next().unwrap();
                self.state.current_player = next_player.id.clone();
                self.state.turns += 1;
                let effects = vec![
                    Effect::EndTurn {
                        player_id: current_player,
                    },
                    Effect::StartTurn {
                        player_id: next_player.id.clone(),
                    },
                ];
                self.state.effects.extend(effects);
            }
        }

        // Move attached artifacts to the same zone as the unit they are attached to
        let attached_artifacts: Vec<(uuid::Uuid, uuid::Uuid)> = self
            .state
            .cards
            .iter()
            .filter(|c| c.is_artifact())
            .filter(|c| c.get_artifact_base().unwrap().attached_to.is_some())
            .map(|c| {
                (
                    c.get_id().clone(),
                    c.get_artifact_base().unwrap().attached_to.unwrap().clone(),
                )
            })
            .collect();
        for (artifact_id, unit_id) in attached_artifacts {
            let unit = self.state.get_card(&unit_id).unwrap();
            let zone = unit.get_zone().clone();
            let artifact = self.state.get_card_mut(&artifact_id).unwrap();
            artifact.set_zone(zone);
        }

        self.send_sync().await?;
        Ok(())
    }

    fn renderables_from_cards(&self) -> Vec<RenderableCard> {
        self.state
            .cards
            .iter()
            .map(|c| RenderableCard {
                id: c.get_id().clone(),
                name: c.get_name().to_string(),
                owner_id: c.get_owner_id().clone(),
                tapped: c.is_tapped(),
                edition: c.get_edition().clone(),
                zone: c.get_zone().clone(),
                card_type: c.get_card_type().clone(),
                modifiers: c.get_modifiers(&self.state),
                plane: c.get_plane().clone(),
                damage_taken: c.get_damage_taken(),
            })
            .collect()
    }

    pub async fn send_sync(&self) -> anyhow::Result<()> {
        let msg = ServerMessage::Sync {
            cards: self.renderables_from_cards(),
            resources: self.state.resources.clone(),
            current_player: self.state.current_player.clone(),
        };

        self.broadcast(&msg).await?;
        Ok(())
    }

    async fn send(stream: Arc<Mutex<OwnedWriteHalf>>, message: &ServerMessage) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message())?;
        let mut stream = stream.lock().await;
        stream.write_all(&bytes).await.unwrap();

        Ok(())
    }

    async fn send_message(&self, message: &ServerMessage, stream: Arc<Mutex<OwnedWriteHalf>>) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message())?;
        let mut stream = stream.lock().await;
        stream.write_all(&bytes).await?;

        Ok(())
    }

    pub async fn broadcast(&self, message: &ServerMessage) -> anyhow::Result<()> {
        for stream in self.streams.values() {
            self.send_message(message, Arc::clone(stream)).await?;
        }
        Ok(())
    }

    pub fn draw_initial_six(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for player in &self.players {
            effects.push(Effect::DrawSite {
                player_id: player.id.clone(),
                count: 3,
            });

            effects.push(Effect::DrawSpell {
                player_id: player.id.clone(),
                count: 3,
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
                player_id: player_id.clone(),
                card_id: avatar_id,
                from: Zone::Spellbook,
                to: ZoneQuery::Specific {
                    id: uuid::Uuid::new_v4(),
                    zone: Zone::Realm(square),
                },
                tap: false,
                plane: Plane::Surface,
                through_path: None,
            });
        }
        effects
    }

    pub async fn process_effects(&mut self) -> anyhow::Result<()> {
        while !self.state.effects.is_empty() {
            if self.state.waiting_for_input {
                return Ok(());
            }

            let effect = self.state.effects.pop_back();
            if let Some(effect) = effect {
                effect.apply(&mut self.state).await?;
                if let Some(description) = effect.description(&self.state).await {
                    self.broadcast(&ServerMessage::LogEvent {
                        id: uuid::Uuid::new_v4(),
                        description,
                        datetime: Utc::now(),
                    })
                    .await?;
                }
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
