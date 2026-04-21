use crate::{
    card::{Ability, AdditionalCost, Aura, CardType, Cost, Region, Zone},
    effect::Effect,
    error::GameError,
    evaluation,
    networking::{
        client::Client,
        message::{ClientMessage, ServerMessage},
    },
    query::{QueryCache, ZoneQuery},
    state::{CardQuery, LoggedEffect, Phase, PlayerWithDeck, State},
};
use async_channel::{Receiver, Sender};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, iter::Sum, sync::Arc};
use tokio::{net::tcp::OwnedWriteHalf, sync::Mutex};

pub type PlayerId = uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SoundEffect {
    DrawCard,
    PlayCard,
    Shuffle,
}

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

    pub fn rotate(&self, times: u8) -> anyhow::Result<Direction> {
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
        if let Some(idx) = directions.iter().position(|d| d == self) {
            let new_idx = (idx + times as usize) % directions.len();
            return Ok(directions[new_idx].clone());
        }

        Err(anyhow::anyhow!("Invalid direction for rotation"))
    }
}

pub const CARDINAL_DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

pub async fn pick_card_with_preview(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[uuid::Uuid],
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    pick_card_with_options(player_id, card_ids, card_ids, false, state, prompt).await
}

pub async fn pick_card_with_options(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[uuid::Uuid],
    pickable_card_ids: &[uuid::Uuid],
    block_opponent: bool,
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            cards: card_ids.to_vec(),
            pickable_cards: pickable_card_ids.to_vec(),
            preview: true,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    let card = match msg {
        ClientMessage::PickCard { card_id, .. } => Ok(card_id),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => unreachable!(),
    };

    if block_opponent {
        resume(&opponent_id, state).await?;
    }

    card
}

pub async fn distribute_damage(
    player_id: impl AsRef<PlayerId>,
    attacker: &uuid::Uuid,
    amount: u16,
    defenders: &[uuid::Uuid],
    state: &State,
) -> anyhow::Result<HashMap<uuid::Uuid, u16>> {
    state
        .get_sender()
        .send(ServerMessage::DistributeDamage {
            player_id: *player_id.as_ref(),
            attacker: *attacker,
            defenders: defenders.to_vec(),
            damage: amount,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::ResolveCombat {
            damage_assignment, ..
        } => Ok(damage_assignment),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => unreachable!(),
    }
}

pub async fn pick_cards(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[uuid::Uuid],
    state: &State,
    prompt: &str,
) -> anyhow::Result<Vec<uuid::Uuid>> {
    state
        .get_sender()
        .send(ServerMessage::PickCards {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            cards: card_ids.to_vec(),
            preview: false,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::PickCards { card_ids, .. } => Ok(card_ids),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => unreachable!(),
    }
}

pub async fn reveal_cards(
    player_id: impl AsRef<PlayerId>,
    preview_cards: &[uuid::Uuid],
    state: &State,
    prompt: &str,
) -> anyhow::Result<()> {
    state
        .get_sender()
        .send(ServerMessage::RevealCards {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            cards: preview_cards.to_vec(),
            action: None,
        })
        .await?;

    Ok(())
}

pub async fn take_action(
    player_id: impl AsRef<PlayerId>,
    preview_cards: &[uuid::Uuid],
    state: &State,
    prompt: &str,
    action: &str,
) -> anyhow::Result<bool> {
    state
        .get_sender()
        .send(ServerMessage::RevealCards {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            cards: preview_cards.to_vec(),
            action: Some(action.to_string()),
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::ResolveAction { take_action, .. } => Ok(take_action),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        n => unreachable!("expected ResolveAction, got {:?}", n),
    }
}
pub async fn pick_card(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[uuid::Uuid],
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            cards: card_ids.to_vec(),
            pickable_cards: card_ids.to_vec(),
            preview: false,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    let card = match msg {
        ClientMessage::PickCard { card_id, .. } => Ok(card_id),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => unreachable!(),
    };

    resume(&opponent_id, state).await?;
    card
}

pub async fn pick_action<'a>(
    player_id: impl AsRef<PlayerId>,
    actions: &'a [Box<dyn ActivatedAbility>],
    state: &State,
    prompt: &str,
    anchor_on_cursor: bool,
) -> anyhow::Result<&'a Box<dyn ActivatedAbility>> {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            actions: actions.iter().map(|c| c.get_name().to_string()).collect(),
            anchor_on_cursor,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::PickAction { action_idx, .. } => Ok(&actions[action_idx]),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickAction, got {:?}", msg),
    }
}

pub async fn resume(player_id: &PlayerId, state: &State) -> anyhow::Result<()> {
    state
        .get_sender()
        .send(ServerMessage::Resume {
            player_id: *player_id,
        })
        .await?;

    Ok(())
}

pub async fn wait_for_opponent(
    player_id: &PlayerId,
    state: &State,
    prompt: impl AsRef<str>,
) -> anyhow::Result<()> {
    state
        .get_sender()
        .send(ServerMessage::Wait {
            player_id: *player_id,
            prompt: prompt.as_ref().to_string(),
        })
        .await?;

    Ok(())
}

pub async fn yes_or_no(
    player_id: impl AsRef<PlayerId>,
    state: &State,
    prompt: impl AsRef<str>,
) -> anyhow::Result<bool> {
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;

    let options = [BaseOption::Yes, BaseOption::No];
    let option_labels = options
        .iter()
        .map(|o| o.to_string())
        .collect::<Vec<String>>();
    let choice = pick_option(player_id, &option_labels, state, prompt, false).await?;

    resume(&opponent_id, state).await?;
    Ok(options[choice] == BaseOption::Yes)
}

pub async fn pick_amount(
    player_id: impl AsRef<PlayerId>,
    min_amount: u8,
    max_amount: u8,
    state: &State,
    prompt: impl AsRef<str>,
) -> anyhow::Result<u8> {
    state
        .get_sender()
        .send(ServerMessage::PickAmount {
            prompt: prompt.as_ref().to_string(),
            player_id: *player_id.as_ref(),
            min_amount,
            max_amount,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::PickAmount { amount, .. } => Ok(amount),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickAction, got {:?}", msg),
    }
}

pub async fn pick_option(
    player_id: impl AsRef<PlayerId>,
    options: &[String],
    state: &State,
    prompt: impl AsRef<str>,
    anchor_on_cursor: bool,
) -> anyhow::Result<usize> {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.as_ref().to_string(),
            player_id: *player_id.as_ref(),
            actions: options.to_vec(),
            anchor_on_cursor,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::PickAction { action_idx, .. } => Ok(action_idx),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickAction, got {:?}", msg),
    }
}

pub async fn pick_path(
    player_id: impl AsRef<PlayerId>,
    paths: &[Vec<Zone>],
    state: &State,
    prompt: &str,
) -> anyhow::Result<Vec<Zone>> {
    state
        .get_sender()
        .send(ServerMessage::PickPath {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            paths: paths.to_vec(),
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::PickPath { path, .. } => Ok(path),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickPath, got {:?}", msg),
    }
}

pub async fn pick_zone_group(
    player_id: impl AsRef<PlayerId>,
    groups: &[Vec<Zone>],
    state: &State,
    block_opponent: bool,
    prompt: &str,
) -> anyhow::Result<Vec<Zone>> {
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZoneGroup {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            groups: groups.to_vec(),
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    let zone = match msg {
        ClientMessage::PickZoneGroup { group_idx, .. } => Ok(groups[group_idx].clone()),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickSquare, got {:?}", msg),
    };

    if block_opponent {
        resume(&opponent_id, state).await?;
    }

    zone
}

pub async fn pick_zone_near(
    player_id: impl AsRef<PlayerId>,
    zone: &Zone,
    state: &State,
    block_opponent: bool,
    prompt: &str,
) -> anyhow::Result<Zone> {
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZone {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            zones: zone.get_nearby(),
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    let zone = match msg {
        ClientMessage::PickZone { zone, .. } => Ok(zone),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickSquare, got {:?}", msg),
    };

    if block_opponent {
        resume(&opponent_id, state).await?;
    }

    zone
}

pub async fn pick_zone(
    player_id: impl AsRef<PlayerId>,
    zones: &[Zone],
    state: &State,
    block_opponent: bool,
    prompt: &str,
) -> anyhow::Result<Zone> {
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZone {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            zones: zones.to_vec(),
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    let zone = match msg {
        ClientMessage::PickZone { zone, .. } => Ok(zone),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickSquare, got {:?}", msg),
    };

    if block_opponent {
        resume(&opponent_id, state).await?;
    }

    zone
}

// Sends a ForceSync message to the specified player with the provided game state. This can be used
// to temporarily override the state of the game for the player, which is useful in cases where the
// state needs to be mutated as part of the card resolution process.
pub async fn force_sync(player_id: impl AsRef<PlayerId>, state: &State) -> anyhow::Result<()> {
    let sync_msg = state.into_sync()?;
    match sync_msg {
        ServerMessage::Sync {
            cards,
            resources,
            current_player,
            health,
            ..
        } => {
            state
                .get_sender()
                .send(ServerMessage::ForceSync {
                    player_id: *player_id.as_ref(),
                    cards,
                    resources,
                    current_player,
                    health,
                })
                .await?;
        }
        _ => panic!("expected Sync message, got {:?}", sync_msg),
    }

    Ok(())
}

pub async fn pick_direction(
    player_id: impl AsRef<PlayerId>,
    directions: &[Direction],
    state: &State,
    prompt: &str,
) -> anyhow::Result<Direction> {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.to_string(),
            player_id: *player_id.as_ref(),
            actions: directions.iter().map(|c| c.get_name()).collect(),
            anchor_on_cursor: false,
        })
        .await?;

    let board_flipped = &state.player_one != player_id.as_ref();
    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::PickAction { action_idx, .. } => {
            Ok(directions[action_idx].normalise(board_flipped))
        }
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickAction, got {:?}", msg),
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

#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub fire: u8,
    pub air: u8,
    pub earth: u8,
    pub water: u8,
}

#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub struct ThresholdsDiff {
    pub fire: i8,
    pub air: i8,
    pub earth: i8,
    pub water: i8,
}

impl ThresholdsDiff {
    pub fn negate(&self) -> ThresholdsDiff {
        ThresholdsDiff {
            fire: -self.fire,
            air: -self.air,
            earth: -self.earth,
            water: -self.water,
        }
    }
}

impl From<&Thresholds> for ThresholdsDiff {
    fn from(val: &Thresholds) -> Self {
        ThresholdsDiff {
            fire: val.fire as i8,
            air: val.air as i8,
            earth: val.earth as i8,
            water: val.water as i8,
        }
    }
}

impl std::ops::Mul<&Thresholds> for u8 {
    type Output = Thresholds;

    fn mul(self, rhs: &Thresholds) -> Thresholds {
        Thresholds {
            fire: rhs.fire.saturating_mul(self),
            air: rhs.air.saturating_mul(self),
            earth: rhs.earth.saturating_mul(self),
            water: rhs.water.saturating_mul(self),
        }
    }
}

impl std::ops::Add<&ThresholdsDiff> for &Thresholds {
    type Output = Thresholds;

    fn add(self, other: &ThresholdsDiff) -> Thresholds {
        Thresholds {
            fire: other.fire.saturating_add_unsigned(self.fire) as u8,
            air: other.air.saturating_add_unsigned(self.air) as u8,
            earth: other.earth.saturating_add_unsigned(self.earth) as u8,
            water: other.water.saturating_add_unsigned(self.water) as u8,
        }
    }
}

impl std::ops::Add<&Thresholds> for &Thresholds {
    type Output = Thresholds;

    fn add(self, other: &Thresholds) -> Thresholds {
        Thresholds {
            fire: self.fire + other.fire,
            air: self.air + other.air,
            earth: self.earth + other.earth,
            water: self.water + other.water,
        }
    }
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

impl From<&str> for Thresholds {
    fn from(val: &str) -> Self {
        Thresholds::parse(val)
    }
}

impl Thresholds {
    pub const ZERO: Self = Thresholds {
        fire: 0,
        air: 0,
        earth: 0,
        water: 0,
    };

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

    pub fn element(&self, element: &Element) -> u8 {
        match element {
            Element::Fire => self.fire,
            Element::Air => self.air,
            Element::Earth => self.earth,
            Element::Water => self.water,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    pub mana: u8,
    pub thresholds: Thresholds,
}

pub fn are_adjacent(square1: &Zone, square2: &Zone) -> bool {
    get_adjacent_zones(square1).contains(square2)
}

pub fn are_nearby(square1: &Zone, square2: &Zone) -> bool {
    get_nearby_zones(square1).contains(square2)
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
            adjacent.retain(|s| s.get_square().unwrap_or(0) <= 20);
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
                    0 => vec![
                        Zone::Realm(sq.saturating_add(4)),
                        Zone::Realm(sq.saturating_sub(6)),
                    ],
                    1 => vec![
                        Zone::Realm(sq.saturating_sub(4)),
                        Zone::Realm(sq.saturating_add(6)),
                    ],
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
                if let Zone::Intersection(isqs) = &intersection
                    && isqs != sqs
                    && isqs.iter().any(|sq| sqs.contains(sq))
                {
                    nearby.push(intersection.clone());
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
            adjacent.retain(|s| s.get_square().unwrap_or(0) <= 20);
            adjacent.retain(|s| s.get_square().unwrap_or(0) > 0);
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
                intersections.push(Zone::Intersection(
                    locs.iter().map(|l| l.saturating_sub(1)).collect(),
                ));
            }

            if locs[0] > 5 {
                intersections.push(Zone::Intersection(
                    locs.iter().map(|l| l.saturating_sub(5)).collect(),
                ));
            }

            intersections
        }
        _ => vec![],
    }
}

/// Returns all zones reachable by a chess knight's move (L-shape) from `zone`.
/// The realm is a 5×4 grid with squares numbered 1–20 (row-major, 5 per row).
pub fn get_knight_move_zones(zone: &Zone) -> Vec<Zone> {
    let sq = match zone.get_square() {
        Some(s) => s as i16,
        None => return vec![],
    };
    let col = ((sq - 1) % 5) + 1; // 1-5
    let row = ((sq - 1) / 5) + 1; // 1-4

    #[rustfmt::skip]
    let offsets: [(i16, i16); 8] = [
        (1, 2),  (-1, 2), (1, -2), (-1, -2),
        (2, 1),  (-2, 1), (2, -1), (-2, -1),
    ];
    offsets
        .iter()
        .map(|(dc, dr)| (col + dc, row + dr))
        .filter(|(c, r)| *c >= 1 && *c <= 5 && *r >= 1 && *r <= 4)
        .map(|(c, r)| Zone::Realm(((r - 1) * 5 + c) as u8))
        .collect()
}

pub trait CloneBoxedAction {
    fn clone_boxed_action(&self) -> Box<dyn ActivatedAbility>;
}

impl<T> CloneBoxedAction for T
where
    T: 'static + ActivatedAbility + Clone,
{
    fn clone_boxed_action(&self) -> Box<dyn ActivatedAbility> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
pub trait ActivatedAbility: std::fmt::Debug + Send + Sync + CloneBoxedAction {
    fn get_name(&self) -> String;

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>>;

    fn can_activate(
        &self,
        _card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn get_cost(&self, _card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::ZERO.clone())
    }
}

impl Clone for Box<dyn ActivatedAbility> {
    fn clone(&self) -> Box<dyn ActivatedAbility> {
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
        damage: u16,
        piercing: bool,
    },
    SelectingAction {
        player_id: PlayerId,
        actions: Vec<Box<dyn ActivatedAbility>>,
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

#[derive(Debug, PartialEq)]
pub enum BaseOption {
    Yes,
    No,
}

impl std::fmt::Display for BaseOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseOption::Yes => write!(f, "Yes"),
            BaseOption::No => write!(f, "No"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CancelAction;

#[async_trait::async_trait]
impl ActivatedAbility for CancelAction {
    fn get_name(&self) -> String {
        "Cancel".to_string()
    }

    async fn on_select(
        &self,
        _card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub enum BaseAction {
    DrawSite,
    DrawSpell,
    Cancel,
}

impl BaseAction {
    pub fn get_name(&self) -> &str {
        match self {
            BaseAction::Cancel => "Cancel",
            BaseAction::DrawSite => "Draw Site",
            BaseAction::DrawSpell => "Draw Spell",
        }
    }

    pub async fn on_select(&self, player_id: &PlayerId, _: &State) -> anyhow::Result<Vec<Effect>> {
        match self {
            BaseAction::DrawSite => Ok(vec![Effect::DrawSite {
                player_id: *player_id,
                count: 1,
            }]),
            BaseAction::DrawSpell => Ok(vec![Effect::DrawSpell {
                player_id: *player_id,
                count: 1,
            }]),
            BaseAction::Cancel => Ok(vec![]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AvatarAction {
    PlaySite,
    DrawSite,
}

#[async_trait::async_trait]
impl ActivatedAbility for AvatarAction {
    fn get_name(&self) -> String {
        match self {
            AvatarAction::PlaySite => "Play Site".to_string(),
            AvatarAction::DrawSite => "Draw Site".to_string(),
        }
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        match self {
            AvatarAction::PlaySite | AvatarAction::DrawSite => {
                Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
            }
        }
    }

    async fn on_select(
        &self,
        _card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            AvatarAction::PlaySite => {
                let cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone() == &Zone::Hand)
                    .filter(|c| c.get_owner_id() == player_id)
                    .map(|c| *c.get_id())
                    .collect();
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let picked_card = state.get_card(&picked_card_id);
                let zones = picked_card.get_valid_play_zones(state, player_id)?;
                let prompt = "Pick a zone to play the site";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                Ok(vec![Effect::PlayCard {
                    player_id: *player_id,
                    card_id: picked_card_id,
                    zone: zone.clone().into(),
                }])
            }
            AvatarAction::DrawSite => Ok(vec![Effect::DrawSite {
                player_id: *player_id,
                count: 1,
            }]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnitAction {
    Move,
    Attack,
    RangedAttack,
    Burrow,
    Submerge,
    Surface,
    PickUpArtifact {
        artifact_id: uuid::Uuid,
        artifact_name: String,
    },
    DropArtifact {
        artifact_id: uuid::Uuid,
        artifact_name: String,
    },
    PickUpMinion,
    DropMinion,
}

#[async_trait::async_trait]
impl ActivatedAbility for UnitAction {
    fn get_name(&self) -> String {
        match self {
            UnitAction::Move => "Move".to_string(),
            UnitAction::Attack => "Attack".to_string(),
            UnitAction::RangedAttack => "Ranged Attack".to_string(),
            UnitAction::Burrow => "Burrow".to_string(),
            UnitAction::Submerge => "Submerge".to_string(),
            UnitAction::Surface => "Surface".to_string(),
            UnitAction::PickUpArtifact { artifact_name, .. } => {
                format!("Pick Up {}", artifact_name)
            }
            UnitAction::DropArtifact { artifact_name, .. } => format!("Drop {}", artifact_name),
            UnitAction::PickUpMinion => "Pick Up Minion".to_string(),
            UnitAction::DropMinion => "Drop Minion".to_string(),
        }
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        let cost = match self {
            UnitAction::Move
            | UnitAction::Attack
            | UnitAction::RangedAttack
            | UnitAction::Burrow
            | UnitAction::Submerge
            | UnitAction::Surface
            | UnitAction::PickUpArtifact { .. }
            | UnitAction::DropArtifact { .. }
            | UnitAction::PickUpMinion
            | UnitAction::DropMinion => Cost::additional_only(AdditionalCost::tap(card_id)),
        };

        Ok(cost)
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            UnitAction::RangedAttack => {
                let card = state.get_card(card_id);
                let cards = card.get_valid_attack_targets(state, true);
                let prompt = "Pick a unit to attack";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                Ok(vec![Effect::RangedStrike {
                    striker_id: *card_id,
                    target_id: picked_card_id,
                }])
            }
            UnitAction::Attack => {
                let attacker = state.get_card(card_id);
                let cards = attacker.get_valid_attack_targets(state, false);
                let prompt = "Pick a unit to attack";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let attacked = state.get_card(&picked_card_id);

                let opponent = state
                    .players
                    .iter()
                    .find(|p| &p.id != player_id)
                    .ok_or(anyhow::anyhow!("opponent not found"))?;
                let possible_defenders = state.get_defenders_for_attack(&picked_card_id);
                if !possible_defenders.is_empty() {
                    wait_for_opponent(
                        player_id,
                        state,
                        "Wait for opponent to choose whether to defend".to_string(),
                    )
                    .await?;

                    let defend = yes_or_no(
                        &opponent.id,
                        state,
                        format!(
                            "{} attacks {}, defend?",
                            attacker.get_name(),
                            attacked.get_name()
                        ),
                    )
                    .await?;
                    resume(player_id, state).await?;

                    if defend {
                        let defenders = pick_cards(
                            &opponent.id,
                            &possible_defenders,
                            state,
                            "Pick units to defend with",
                        )
                        .await?;
                        match defenders.len() {
                            // If no defenders are picked, proceed with the original attack.
                            0 => {
                                return Ok(vec![Effect::Attack {
                                    attacker_id: *card_id,
                                    defender_id: picked_card_id,
                                }]);
                            }
                            // If a single defender is picked, change the attack to target the
                            // defender.
                            1 => {
                                let defender_id = defenders[0];
                                let defender = state.get_card(&defender_id);
                                return Ok(vec![
                                    // Return the attack effect first so that MoveCard is applied
                                    // before attack and the attack happens on the correct zone.
                                    Effect::Attack {
                                        attacker_id: *card_id,
                                        defender_id,
                                    },
                                    Effect::MoveCard {
                                        player_id: opponent.id,
                                        card_id: defender_id,
                                        from: defender.get_zone().clone(),
                                        to: ZoneQuery::from_zone(attacker.get_zone().clone()),
                                        tap: true,
                                        region: attacker.get_region(state).clone(),
                                        through_path: None,
                                    },
                                ]);
                            }
                            _ => {
                                let mut effects = defenders
                                    .iter()
                                    .flat_map(|defender_id| {
                                        let defender_zone =
                                            state.get_card(defender_id).get_zone().clone();
                                        vec![Effect::MoveCard {
                                            player_id: opponent.id,
                                            card_id: *defender_id,
                                            from: defender_zone,
                                            to: ZoneQuery::from_zone(attacker.get_zone().clone()),
                                            tap: true,
                                            region: attacker.get_region(state).clone(),
                                            through_path: None,
                                        }]
                                    })
                                    .collect::<Vec<Effect>>();

                                wait_for_opponent(
                                    &opponent.id,
                                    state,
                                    "Wait for opponent to distribute damage".to_string(),
                                )
                                .await?;

                                let damage_distribution = distribute_damage(
                                    player_id,
                                    card_id,
                                    attacker.get_power(state)?.unwrap_or_default(),
                                    &defenders,
                                    state,
                                )
                                .await?;

                                for (defender_id, damage) in damage_distribution {
                                    effects.push(Effect::TakeDamage {
                                        card_id: defender_id,
                                        from: *card_id,
                                        damage,
                                        is_strike: false,
                                    });

                                    let defender = state.get_card(&defender_id);
                                    effects.extend(defender.on_defend(state, attacker.get_id())?);
                                }

                                resume(&opponent.id, state).await?;
                                effects.reverse();
                                return Ok(effects);
                            }
                        }
                    }
                }

                Ok(vec![Effect::Attack {
                    attacker_id: *card_id,
                    defender_id: picked_card_id,
                }])
            }
            UnitAction::Move => {
                let card = state.get_card(card_id);
                let zones = card.get_valid_move_zones(state)?;
                let prompt = "Pick a zone to move to";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                let paths = card.get_valid_move_paths(state, &zone)?;
                let path = if paths.len() > 1 {
                    let prompt = "Pick a path to move along";
                    pick_path(player_id, &paths, state, prompt).await?
                } else {
                    paths
                        .first()
                        .ok_or(anyhow::anyhow!("no paths found"))?
                        .to_vec()
                };

                let can_be_intercepted = !card.has_ability(state, &Ability::Uninterceptable);
                let opponent = state
                    .players
                    .iter()
                    .find(|p| &p.id != player_id)
                    .ok_or(anyhow::anyhow!("opponent not found"))?;
                let interceptors = state.get_interceptors_for_move(&path, &opponent.id);
                let mut interceptor: Option<(uuid::Uuid, Zone)> = None;
                if can_be_intercepted && !interceptors.is_empty() {
                    let mut options = interceptors
                        .iter()
                        .map(|(id, zone)| {
                            let interceptor_card = state.get_card(id);
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
                        "Wait for opponent to choose whether to intersect".to_string(),
                    )
                    .await?;

                    let action_idx = pick_option(
                        &opponent.id,
                        &options,
                        state,
                        format!("Intercept {} with...", card.get_name()),
                        false,
                    )
                    .await?;
                    if action_idx < interceptors.len() {
                        interceptor = Some(interceptors[action_idx].clone());
                    }

                    resume(player_id, state).await?;
                }

                let mut effects = Vec::new();
                for (idx, zone) in path.iter().enumerate() {
                    if idx + 1 >= path.len() {
                        break;
                    }

                    let to_zone = path[idx + 1].clone();
                    effects.push(Effect::MoveCard {
                        player_id: *player_id,
                        card_id: *card_id,
                        from: zone.clone(),
                        to: ZoneQuery::from_zone(to_zone.clone()),
                        tap: true,
                        region: card.get_region(state).clone(),
                        through_path: Some(path.clone()),
                    });

                    if let Some((interceptor_id, zone)) = &interceptor {
                        if &to_zone != zone {
                            continue;
                        }

                        let interceptor_card = state.get_card(interceptor_id);
                        effects.push(Effect::MoveCard {
                            player_id: opponent.id,
                            card_id: *interceptor_id,
                            from: interceptor_card.get_zone().clone(),
                            to: ZoneQuery::from_zone(zone.clone()),
                            tap: true,
                            region: card.get_region(state).clone(),
                            through_path: Some(path.clone()),
                        });
                        effects.push(Effect::Attack {
                            attacker_id: *interceptor_id,
                            defender_id: *card_id,
                        });

                        break;
                    }
                }

                effects.reverse();
                Ok(effects)
            }
            UnitAction::Burrow => Ok(vec![Effect::SetCardRegion {
                card_id: *card_id,
                region: Region::Underground,
                tap: true,
            }]),
            UnitAction::Submerge => Ok(vec![Effect::SetCardRegion {
                card_id: *card_id,
                region: Region::Underwater,
                tap: true,
            }]),
            UnitAction::Surface => Ok(vec![Effect::SetCardRegion {
                card_id: *card_id,
                region: Region::Surface,
                tap: true,
            }]),
            UnitAction::PickUpArtifact { artifact_id, .. } => Ok(vec![Effect::SetBearer {
                card_id: *artifact_id,
                bearer_id: Some(*card_id),
            }]),
            UnitAction::DropArtifact { artifact_id, .. } => Ok(vec![Effect::SetBearer {
                card_id: *artifact_id,
                bearer_id: None,
            }]),
            UnitAction::PickUpMinion => {
                let card = state.get_card(card_id);
                let minions: Vec<uuid::Uuid> = CardQuery::new()
                    .minions()
                    .in_zone(card.get_zone())
                    .id_not_in(vec![*card.get_id()])
                    .all(state)
                    .into_iter()
                    .filter(|minion_id| {
                        state
                            .get_card(minion_id)
                            .get_bearer_id()
                            .unwrap_or_default()
                            .is_none()
                    })
                    .collect();
                let picked =
                    pick_cards(player_id, &minions, state, "Pick minions to carry").await?;
                Ok(picked
                    .into_iter()
                    .map(|minion_id| Effect::SetBearer {
                        card_id: minion_id,
                        bearer_id: Some(*card_id),
                    })
                    .collect())
            }
            UnitAction::DropMinion => {
                let minions = state
                    .cards
                    .iter()
                    .filter(|minion| minion.is_minion())
                    .filter(|minion| minion.get_bearer_id().unwrap_or_default() == Some(*card_id))
                    .collect::<Vec<_>>();
                let picked = pick_cards(
                    player_id,
                    &minions.iter().map(|c| *c.get_id()).collect::<Vec<_>>(),
                    state,
                    "Drop carried minions",
                )
                .await?;
                Ok(picked
                    .into_iter()
                    .map(|minion_id| Effect::SetBearer {
                        card_id: minion_id,
                        bearer_id: None,
                    })
                    .collect())
            }
        }
    }
}

pub struct Game {
    pub id: uuid::Uuid,
    pub state: State,
    /// When `true` every `Sync` message broadcast to clients includes a full
    /// board evaluation so that UIs and AI agents can display/log it.
    pub debug_eval: bool,
    streams: HashMap<PlayerId, Arc<Mutex<OwnedWriteHalf>>>,
    client_receiver: Receiver<ClientMessage>,
    server_receiver: Receiver<ServerMessage>,
}

impl Game {
    pub fn new(
        players_with_streams: Vec<(PlayerWithDeck, Arc<Mutex<OwnedWriteHalf>>)>,
        receiver: Receiver<ClientMessage>,
        server_sender: Sender<ServerMessage>,
        server_receiver: Receiver<ServerMessage>,
    ) -> Self {
        let game_id = uuid::Uuid::new_v4();
        let mut streams = HashMap::new();
        for player in &players_with_streams {
            streams.insert(player.0.player.id, player.1.clone());
        }
        let players = players_with_streams.into_iter().map(|p| p.0).collect();

        Game {
            id: game_id,
            streams,
            state: State::new(game_id, players, server_sender.clone(), receiver.clone()),
            client_receiver: receiver,
            server_receiver,
            debug_eval: false,
        }
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.state
            .effects
            .extend(self.place_avatars().into_iter().map(|e| e.into()));
        self.state
            .effects
            .extend(self.draw_initial_six().into_iter().map(|e| e.into()));

        // Process effects before starting the game so players don't see the initial setup in the event log
        self.process_effects().await?;

        self.broadcast(&ServerMessage::GameStarted {
            player1: self.state.players[0].id,
            player2: self.state.players[1].id,
            game_id: self.id,
            cards: self.state.data_from_cards(),
        })
        .await?;
        self.process_effects().await?;
        self.broadcast(&self.make_sync()?).await?;

        let streams = self.streams.clone();
        let receiver = self.server_receiver.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(message) = receiver.recv().await {
                    let stream = streams
                        .get(&message.player_id())
                        .expect("stream to be found");
                    Client::send_to_stream(&message, Arc::clone(stream))
                        .await
                        .expect("message to be sent");
                }
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
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                self.player_disconnected(player_id).await?;
            }
            ClientMessage::ClickCard {
                player_id, card_id, ..
            } => {
                let snapshot = self.state.snapshot();
                let card = snapshot.get_card(card_id);
                // if &card.get_controller_id(&self.state) != player_id {
                //     return Ok(());
                // }

                if player_id != &self.state.current_player {
                    return Ok(());
                }

                if card.is_playable(&self.state, player_id)?
                    && card.is_affordable(&self.state, player_id)?
                {
                    match card.get_card_type() {
                        CardType::Artifact | CardType::Minion | CardType::Aura => {
                            let effects = card.play_mechanic(&self.state, player_id).await?;
                            self.state.queue(effects);
                        }
                        CardType::Magic => {
                            let spellcasters =
                                CardQuery::new().units().can_cast(card_id).all(&self.state);
                            let prompt = "Pick a spellcaster to cast the spell";
                            let caster_id =
                                pick_card(player_id, &spellcasters, &self.state, prompt).await?;
                            let caster = self.state.get_card(&caster_id);
                            self.state.queue_one(Effect::PlayMagic {
                                player_id: *player_id,
                                card_id: *card_id,
                                caster_id,
                                from: caster.get_zone().clone(),
                            });
                        }
                        // Sites are not playable by clicking, they have to be played through avatar
                        // actions, so ignore clicks on them in the hand.
                        CardType::Site => {}
                        // Avatars should not even be playable from any zone.
                        CardType::Avatar => {}
                    }
                }

                if matches!(card.get_zone(), Zone::Realm(_)) {
                    let unit_disabled = card.has_ability(&self.state, &Ability::SummoningSickness);
                    if card.is_unit() && unit_disabled {
                        return Ok(());
                    }

                    let mut actions = card.get_activated_abilities(&self.state)?;
                    actions.retain(|action| {
                        let can_afford = action
                            .get_cost(card_id, &self.state)
                            .and_then(|cost| cost.can_afford(&self.state, player_id))
                            .unwrap_or_default();
                        let can_activate = action
                            .can_activate(card_id, player_id, &self.state)
                            .unwrap_or_default();
                        can_afford && can_activate
                    });

                    if actions.is_empty() {
                        return Ok(());
                    }

                    actions.push(Box::new(CancelAction));
                    let prompt = format!("{}: Pick action", card.get_name());
                    let action =
                        pick_action(player_id, &actions, &self.state, &prompt, true).await?;
                    let cost = action.get_cost(card_id, &self.state)?.clone();
                    let effects = action
                        .on_select(card.get_id(), player_id, &self.state)
                        .await?;
                    // Pay costs after selecting the action to work around scenarios where
                    // the cost includes sacricing the card and the effects involve nearby
                    // cards, which would result in no valid targets, as the card would already
                    // be in the cemetery at the point of execution the action.
                    cost.pay(&mut self.state, player_id).await?;
                    self.state.queue(effects);
                }
            }
            ClientMessage::EndTurn { player_id, .. } => {
                self.state.queue_one(Effect::EndTurn {
                    player_id: *player_id,
                });
            }
            ClientMessage::PickCards {
                card_ids,
                player_id,
                ..
            } if self.state.phase == Phase::Mulligan => {
                let mut deck = self.state.get_player_deck(player_id)?.clone();
                let mut site_count = 0;
                for card_id in card_ids {
                    let card = self.state.get_card_mut(card_id);
                    match card.get_card_type() {
                        CardType::Site => {
                            site_count += 1;
                            deck.sites.push(*card_id);
                            card.set_zone(Zone::Atlasbook);
                        }
                        _ => {
                            deck.spells.push(*card_id);
                            card.set_zone(Zone::Spellbook);
                        }
                    }
                }

                let spell_count = card_ids.len() - site_count;
                deck.rotate_sites(site_count);
                deck.rotate_spells(spell_count);

                let effects = vec![
                    Effect::DrawSite {
                        player_id: *player_id,
                        count: site_count as u8,
                    },
                    Effect::DrawSpell {
                        player_id: *player_id,
                        count: spell_count as u8,
                    },
                ];
                self.state.players_with_accepted_hands.insert(*player_id);
                self.state.queue(effects);
                if self.state.players_with_accepted_hands.len() == self.state.players.len() {
                    self.state.phase = Phase::Main;
                    self.process_effects().await?;
                    self.broadcast(&ServerMessage::MulligansEnded).await?;
                    self.broadcast(&self.make_sync()?).await?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn end_game(&mut self) -> anyhow::Result<()> {
        QueryCache::clear_game_cache(&self.id).await;
        Ok(())
    }

    pub async fn player_disconnected(&mut self, player_id: &PlayerId) -> anyhow::Result<()> {
        self.streams.retain(|pid, _| pid != player_id);
        self.broadcast(&ServerMessage::PlayerDisconnected {
            player_id: *player_id,
        })
        .await?;
        self.end_game().await?;

        Ok(())
    }

    pub async fn process_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        match self.handle_message(message).await {
            Ok(_) => {}
            Err(e) => {
                if let Some(e) = e.downcast_ref::<GameError>() {
                    match e {
                        GameError::PlayerDisconnected(player_id) => {
                            self.player_disconnected(player_id).await?;
                            return Ok(());
                        }
                    }
                } else {
                    println!("Error processing message: {:?}", e);
                }
            }
        }
        self.update().await?;
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        self.process_effects().await?;

        // Move attached artifacts to the same zone as the unit they are attached to
        let attached_artifacts: Vec<(uuid::Uuid, uuid::Uuid)> = self
            .state
            .cards
            .iter()
            .filter(|c| c.is_artifact())
            .filter_map(|c| {
                c.get_base()
                    .bearer
                    .map(|attached_to| (*c.get_id(), attached_to))
            })
            .collect();
        for (artifact_id, unit_id) in attached_artifacts {
            let unit = self.state.get_card(&unit_id);
            let zone = unit.get_zone().clone();
            let artifact = self.state.get_card_mut(&artifact_id);
            artifact.set_zone(zone);
        }

        self.broadcast(&self.make_sync()?).await?;
        Ok(())
    }

    pub async fn broadcast(&self, message: &ServerMessage) -> anyhow::Result<()> {
        for stream in self.streams.values() {
            Client::send_to_stream(message, Arc::clone(stream)).await?;
        }
        Ok(())
    }

    /// Build a `Sync` message for the current state.  When `debug_eval` is
    /// enabled the message also carries a full board evaluation.
    fn make_sync(&self) -> anyhow::Result<ServerMessage> {
        let mut sync = self.state.into_sync()?;
        if self.debug_eval
            && let ServerMessage::Sync {
                ref mut evaluation, ..
            } = sync
        {
            *evaluation = Some(evaluation::evaluate(&self.state));
        }
        Ok(sync)
    }

    pub fn draw_initial_six(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for player in &self.state.players {
            effects.push(Effect::DrawSite {
                player_id: player.id,
                count: 3,
            });

            effects.push(Effect::DrawSpell {
                player_id: player.id,
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
                player_id: *player_id,
                card_id: avatar_id,
                from: Zone::Spellbook,
                to: ZoneQuery::from_zone(Zone::Realm(square)),
                tap: false,
                region: Region::Surface,
                through_path: None,
            });
        }
        effects
    }

    async fn dispell_auras(state: &mut State) -> anyhow::Result<()> {
        let auras: Vec<&dyn Aura> = state.cards.iter().filter_map(|c| c.get_aura()).collect();
        let mut auras_to_dispell = vec![];
        for aura in auras {
            if aura.should_dispell(state)? {
                auras_to_dispell.push(*aura.get_id());
            }
        }

        for aura_id in auras_to_dispell {
            {
                let card = state.get_card_mut(&aura_id);
                card.set_zone(Zone::Cemetery);
            }

            let card = state.get_card(&aura_id);
            let effects = card.deathrite(state, card.get_zone());
            state.queue(effects);
        }

        Ok(())
    }

    pub async fn process_effects(&mut self) -> anyhow::Result<()> {
        while !self.state.effects.is_empty() {
            let effect = self.state.effects.pop_back();
            if let Some(effect) = effect {
                match effect.apply(&mut self.state).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error applying effect {:?}: {:?}", effect, e);
                    }
                }

                let description = effect.description(&self.state).await.ok().flatten();

                // Show the card face to all players when a card is played from hand.
                // When CardPlayed is sent, skip the LogEvent to avoid showing the
                // same description twice on the client.
                let is_card_played = effect.played_card_id().is_some();
                if let Some(card_id) = effect.played_card_id() {
                    self.broadcast(&ServerMessage::CardPlayed {
                        card_id,
                        description: description.clone().unwrap_or_default(),
                    })
                    .await?;
                }

                if !is_card_played && let Some(desc) = description {
                    self.broadcast(&ServerMessage::LogEvent {
                        id: uuid::Uuid::new_v4(),
                        description: desc,
                        datetime: Utc::now(),
                    })
                    .await?;
                }

                if let Ok(Some(sound_effect)) = effect.sound_effect().await {
                    self.broadcast(&ServerMessage::PlaySoundEffect {
                        player_id: None,
                        sound_effect,
                    })
                    .await?;
                }

                // Move the effect to the effect log so we can keep track of what has happened in
                // the game.
                self.state
                    .effect_log
                    .push(LoggedEffect::new(effect.clone(), self.state.turns));
                self.state.compute_world_effects().await?;

                Self::dispell_auras(&mut self.state).await?;
                self.broadcast(&self.make_sync()?).await?;
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
