use crate::{
    card::{Ability, Aura, CardType, Cost, Region, Zone},
    effect::Effect,
    error::GameError,
    networking::{
        client::Client,
        message::{ClientMessage, ServerMessage},
    },
    query::{QueryCache, ZoneQuery},
    state::{PlayerWithDeck, State},
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

pub const CARDINAL_DIRECTIONS: [Direction; 4] = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];

pub async fn pick_card_with_preview(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[uuid::Uuid],
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            player_id: player_id.as_ref().clone(),
            cards: card_ids.to_vec(),
            preview: true,
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickCard { card_id, .. } => break Ok(card_id),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => unreachable!(),
        }
    }
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
            player_id: player_id.as_ref().clone(),
            attacker: attacker.clone(),
            defenders: defenders.to_vec(),
            damage: amount,
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::ResolveCombat { damage_assignment, .. } => break Ok(damage_assignment),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => unreachable!(),
        }
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
            player_id: player_id.as_ref().clone(),
            cards: card_ids.to_vec(),
            preview: false,
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickCards { card_ids, .. } => break Ok(card_ids),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => unreachable!(),
        }
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
            player_id: player_id.as_ref().clone(),
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
            player_id: player_id.as_ref().clone(),
            cards: preview_cards.to_vec(),
            action: Some(action.to_string()),
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::ResolveAction { take_action, .. } => break Ok(take_action),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => unreachable!(),
        }
    }
}
pub async fn pick_card(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[uuid::Uuid],
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            player_id: player_id.as_ref().clone(),
            cards: card_ids.to_vec(),
            preview: false,
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickCard { card_id, .. } => break Ok(card_id),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => unreachable!(),
        }
    }
}

pub async fn pick_action<'a>(
    player_id: impl AsRef<PlayerId>,
    actions: &'a [Box<dyn ActivatedAbility>],
    state: &State,
    prompt: &str,
) -> anyhow::Result<&'a Box<dyn ActivatedAbility>> {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.to_string(),
            player_id: player_id.as_ref().clone(),
            actions: actions.iter().map(|c| c.get_name().to_string()).collect(),
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickAction { action_idx, .. } => break Ok(&actions[action_idx]),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => panic!("expected PickAction, got {:?}", msg),
        }
    }
}

pub async fn resume(player_id: &PlayerId, state: &State) -> anyhow::Result<()> {
    state
        .get_sender()
        .send(ServerMessage::Resume {
            player_id: player_id.clone(),
        })
        .await?;

    Ok(())
}

pub async fn wait_for_opponent(player_id: &PlayerId, state: &State, prompt: impl AsRef<str>) -> anyhow::Result<()> {
    state
        .get_sender()
        .send(ServerMessage::Wait {
            player_id: player_id.clone(),
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
    let options = vec![BaseOption::Yes, BaseOption::No];
    let option_labels = options.iter().map(|o| o.to_string()).collect::<Vec<String>>();
    let choice = pick_option(player_id, &option_labels, state, prompt).await?;
    Ok(options[choice] == BaseOption::Yes)
}

pub async fn pick_option(
    player_id: impl AsRef<PlayerId>,
    options: &[String],
    state: &State,
    prompt: impl AsRef<str>,
) -> anyhow::Result<usize> {
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.as_ref().to_string(),
            player_id: player_id.as_ref().clone(),
            actions: options.to_vec(),
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickAction { action_idx, .. } => break Ok(action_idx),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => panic!("expected PickAction, got {:?}", msg),
        }
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
            player_id: player_id.as_ref().clone(),
            paths: paths.to_vec(),
        })
        .await?;

    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickPath { path, .. } => break Ok(path),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => panic!("expected PickPath, got {:?}", msg),
        }
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
        wait_for_opponent(&opponent_id, state, "Wait for opponent to pick a zone...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZoneGroup {
            prompt: prompt.to_string(),
            player_id: player_id.as_ref().clone(),
            groups: groups.to_vec(),
        })
        .await?;

    let zone = loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickZoneGroup { group_idx, .. } => break Ok(groups[group_idx].clone()),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => panic!("expected PickSquare, got {:?}", msg),
        }
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
        wait_for_opponent(&opponent_id, state, "Wait for opponent to pick a zone...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZone {
            prompt: prompt.to_string(),
            player_id: player_id.as_ref().clone(),
            zones: zones.to_vec(),
        })
        .await?;

    let zone = loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickZone { zone, .. } => break Ok(zone),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
            _ => panic!("expected PickSquare, got {:?}", msg),
        }
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
        } => {
            state
                .get_sender()
                .send(ServerMessage::ForceSync {
                    player_id: player_id.as_ref().clone(),
                    cards: cards,
                    resources: resources,
                    current_player: current_player,
                    health: health,
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
            player_id: player_id.as_ref().clone(),
            actions: directions.iter().map(|c| c.get_name()).collect(),
        })
        .await?;

    let board_flipped = &state.player_one != player_id.as_ref();
    loop {
        let msg = state.get_receiver().recv().await?;
        match msg {
            ClientMessage::PickAction { action_idx, .. } => break Ok(directions[action_idx].normalise(board_flipped)),
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                return Err(GameError::PlayerDisconnected(player_id.clone()).into());
            }
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

impl Into<Thresholds> for &str {
    fn into(self) -> Thresholds {
        Thresholds::parse(self)
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    pub mana: u8,
    pub thresholds: Thresholds,
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
    fn get_name(&self) -> &str;
    async fn on_select(&self, card_id: &uuid::Uuid, player_id: &PlayerId, state: &State)
    -> anyhow::Result<Vec<Effect>>;
    fn get_cost(&self, _card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::zero())
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

impl ToString for BaseOption {
    fn to_string(&self) -> String {
        match self {
            BaseOption::Yes => "Yes".to_string(),
            BaseOption::No => "No".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CancelAction;

#[async_trait::async_trait]
impl ActivatedAbility for CancelAction {
    fn get_name(&self) -> &str {
        "Cancel"
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
                player_id: player_id.clone(),
                count: 1,
            }]),
            BaseAction::DrawSpell => Ok(vec![Effect::DrawSpell {
                player_id: player_id.clone(),
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
    fn get_name(&self) -> &str {
        match self {
            AvatarAction::PlaySite => "Play Site",
            AvatarAction::DrawSite => "Draw Site",
        }
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
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
                    .map(|c| c.get_id().clone())
                    .collect();
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let picked_card = state.get_card(&picked_card_id);
                let zones = picked_card.get_valid_play_zones(state)?;
                let prompt = "Pick a zone to play the site";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                Ok(vec![
                    Effect::PlayCard {
                        player_id: player_id.clone(),
                        card_id: picked_card_id.clone(),
                        zone: zone.clone(),
                    },
                    Effect::TapCard {
                        card_id: card_id.clone(),
                    },
                ])
            }
            AvatarAction::DrawSite => Ok(vec![
                Effect::DrawSite {
                    player_id: player_id.clone(),
                    count: 1,
                },
                Effect::TapCard {
                    card_id: card_id.clone(),
                },
            ]),
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
impl ActivatedAbility for UnitAction {
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
                    attacker_id: card_id.clone(),
                    defender_id: picked_card_id,
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
                        format!("Wait for opponent to choose whether to defend"),
                    )
                    .await?;

                    let defend = yes_or_no(
                        &opponent.id,
                        state,
                        format!("{} attacks {}, defend?", attacker.get_name(), attacked.get_name()),
                    )
                    .await?;
                    resume(player_id, state).await?;

                    if defend {
                        let defenders =
                            pick_cards(&opponent.id, &possible_defenders, state, "Pick units to defend with").await?;
                        match defenders.len() {
                            // If no defenders are picked, proceed with the original attack.
                            0 => {
                                return Ok(vec![Effect::Attack {
                                    attacker_id: card_id.clone(),
                                    defender_id: picked_card_id,
                                }]);
                            }
                            // If a single defender is picked, change the attack to target the
                            // defender.
                            1 => {
                                let defender_id = defenders[0];
                                let defender = state.get_card(&defender_id);
                                return Ok(vec![
                                    Effect::MoveCard {
                                        player_id: opponent.id.clone(),
                                        card_id: defender_id.clone(),
                                        from: defender.get_zone().clone(),
                                        to: ZoneQuery::Specific {
                                            id: uuid::Uuid::new_v4(),
                                            zone: attacker.get_zone().clone(),
                                        },
                                        tap: true,
                                        region: attacker.get_region(state).clone(),
                                        through_path: None,
                                    },
                                    Effect::Attack {
                                        attacker_id: card_id.clone(),
                                        defender_id: defender_id.clone(),
                                    },
                                ]);
                            }
                            _ => {
                                let mut effects = defenders
                                    .iter()
                                    .flat_map(|defender_id| {
                                        let defender_zone = state.get_card(defender_id).get_zone().clone();
                                        vec![Effect::MoveCard {
                                            player_id: opponent.id.clone(),
                                            card_id: defender_id.clone(),
                                            from: defender_zone,
                                            to: ZoneQuery::Specific {
                                                id: uuid::Uuid::new_v4(),
                                                zone: attacker.get_zone().clone(),
                                            },
                                            tap: true,
                                            region: attacker.get_region(state).clone(),
                                            through_path: None,
                                        }]
                                    })
                                    .collect::<Vec<Effect>>();

                                wait_for_opponent(
                                    &opponent.id,
                                    state,
                                    format!("Wait for opponent to distribute damage"),
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
                                        card_id: defender_id.clone(),
                                        from: card_id.clone(),
                                        damage,
                                    });

                                    let defender = state.get_card(&defender_id);
                                    effects.extend(defender.on_defend(state, attacker.get_id())?);
                                }

                                effects.extend(attacker.after_attack(state).await?);
                                resume(&opponent.id, state).await?;
                                effects.reverse();
                                return Ok(effects);
                            }
                        }
                    }
                }

                Ok(vec![Effect::Attack {
                    attacker_id: card_id.clone(),
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
                    paths.first().ok_or(anyhow::anyhow!("no paths found"))?.to_vec()
                };

                let opponent = state
                    .players
                    .iter()
                    .find(|p| &p.id != player_id)
                    .ok_or(anyhow::anyhow!("opponent not found"))?;
                let interceptors = state.get_interceptors_for_move(&path, &opponent.id);
                let mut interceptor: Option<(uuid::Uuid, Zone)> = None;
                if !interceptors.is_empty() {
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
                        format!("Wait for opponent to choose whether to intersect"),
                    )
                    .await?;

                    let action_idx = pick_option(
                        &opponent.id,
                        &options,
                        state,
                        format!("Intercept {} with...", card.get_name()),
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
                        player_id: player_id.clone(),
                        card_id: card_id.clone(),
                        from: zone.clone(),
                        to: ZoneQuery::Specific {
                            id: uuid::Uuid::new_v4(),
                            zone: to_zone.clone(),
                        },
                        tap: true,
                        region: card.get_base().region.clone(),
                        through_path: None,
                    });

                    if let Some((interceptor_id, zone)) = &interceptor {
                        if &to_zone != zone {
                            continue;
                        }

                        let interceptor_card = state.get_card(&interceptor_id);
                        effects.push(Effect::MoveCard {
                            player_id: opponent.id.clone(),
                            card_id: interceptor_id.clone(),
                            from: interceptor_card.get_zone().clone(),
                            to: ZoneQuery::Specific {
                                id: uuid::Uuid::new_v4(),
                                zone: zone.clone(),
                            },
                            tap: true,
                            region: card.get_base().region.clone(),
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
                Ok(effects)
            }
            UnitAction::Burrow => Ok(vec![Effect::Burrow {
                card_id: card_id.clone(),
            }]),
            UnitAction::Submerge => Ok(vec![Effect::Submerge {
                card_id: card_id.clone(),
            }]),
            UnitAction::Defend => Ok(vec![]),
        }
    }
}

pub struct Game {
    pub id: uuid::Uuid,
    pub state: State,
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
            streams.insert(player.0.player.id.clone(), player.1.clone());
        }
        let players = players_with_streams.into_iter().map(|p| p.0).collect();

        Game {
            id: game_id.clone(),
            streams,
            state: State::new(game_id, players, server_sender.clone(), receiver.clone()),
            client_receiver: receiver,
            server_receiver,
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
            player1: self.state.players[0].id.clone(),
            player2: self.state.players[1].id.clone(),
            game_id: self.id.clone(),
            cards: self.state.data_from_cards(),
        })
        .await?;
        self.process_effects().await?;
        self.broadcast(&self.state.into_sync()?).await?;

        let streams = self.streams.clone();
        let receiver = self.server_receiver.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(message) = receiver.recv().await {
                    let stream = streams.get(&message.player_id()).expect("stream to be found");
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
            ClientMessage::ClickCard { player_id, card_id, .. } => {
                let snapshot = self.state.snapshot();
                let card = snapshot.get_card(card_id);
                if &card.get_controller_id(&self.state) != player_id {
                    return Ok(());
                }

                if player_id != &self.state.current_player {
                    return Ok(());
                }

                if let Zone::Hand = card.get_zone() {
                    if !card.get_cost(&self.state)?.can_afford(&self.state, player_id)? {
                        return Ok(());
                    }
                }

                match (card.get_card_type(), card.get_zone()) {
                    (CardType::Artifact, Zone::Hand) => {
                        let units = card
                            .get_artifact()
                            .ok_or(anyhow::anyhow!("artifact card does not implement artifact"))?
                            .get_valid_attach_targets(&self.state);
                        let needs_bearer = self
                            .state
                            .get_card(card_id)
                            .get_artifact()
                            .ok_or(anyhow::anyhow!("artifact card does not implement artifact"))?
                            .needs_bearer(&self.state)?;
                        match needs_bearer {
                            true => {
                                let picked_card_id = pick_card(
                                    player_id,
                                    &units,
                                    &self.state,
                                    format!("Pick a unit to attach {} to", card.get_name()).as_str(),
                                )
                                .await?;
                                self.state
                                    .get_card_mut(card_id)
                                    .get_artifact_base_mut()
                                    .ok_or(anyhow::anyhow!("artifact card has no base artifact component"))?
                                    .bearer = Some(picked_card_id.clone());
                            }
                            false => {
                                let picked_zone = pick_zone(
                                    player_id,
                                    &card.get_valid_play_zones(&self.state)?,
                                    &self.state,
                                    false,
                                    "Pick a zone to play the artifact",
                                )
                                .await?;
                                self.state.effects.push_back(
                                    Effect::PlayCard {
                                        player_id: player_id.clone(),
                                        card_id: card_id.clone(),
                                        zone: picked_zone.clone(),
                                    }
                                    .into(),
                                );
                            }
                        }
                    }
                    (CardType::Minion, Zone::Hand) | (CardType::Aura, Zone::Hand) => {
                        let zones = card.get_valid_play_zones(&self.state)?;
                        let prompt = "Pick a zone to play the card";
                        let zone = pick_zone(player_id, &zones, &self.state, false, prompt).await?;
                        self.state.effects.push_back(
                            Effect::PlayCard {
                                player_id: player_id.clone(),
                                card_id: card_id.clone(),
                                zone: zone.clone(),
                            }
                            .into(),
                        );
                    }
                    (CardType::Magic, Zone::Hand) => {
                        let spellcasters: Vec<uuid::Uuid> = self
                            .state
                            .cards
                            .iter()
                            .filter(|c| c.can_cast(&self.state, card).unwrap_or_default())
                            .map(|c| c.get_id().clone())
                            .collect();
                        let prompt = "Pick a spellcaster to cast the spell";
                        let caster_id = pick_card(player_id, &spellcasters, &self.state, prompt).await?;
                        let caster = self.state.get_card(&caster_id);
                        self.state.effects.push_back(
                            Effect::PlayMagic {
                                player_id: player_id.clone(),
                                card_id: card_id.clone(),
                                caster_id,
                                from: caster.get_zone().clone(),
                            }
                            .into(),
                        );
                    }
                    (_, Zone::Realm(_)) => {
                        let unit_disabled =
                            card.is_tapped() || card.has_ability(&self.state, &Ability::SummoningSickness);
                        if card.is_unit() && unit_disabled {
                            return Ok(());
                        }

                        let mut actions = card.get_activated_abilities(&self.state)?;
                        actions.retain(|action| {
                            action
                                .get_cost(card_id, &self.state)
                                .and_then(|cost| cost.can_afford(&self.state, player_id))
                                .unwrap_or_default()
                        });

                        if actions.is_empty() {
                            return Ok(());
                        }

                        actions.push(Box::new(CancelAction));
                        let prompt = format!("{}: Pick action", card.get_name());
                        let action = pick_action(player_id, &actions, &self.state, &prompt).await?;
                        let cost = action.get_cost(card_id, &self.state)?.clone();
                        cost.pay(&mut self.state, player_id).await?;
                        let effects = action.on_select(card.get_id(), player_id, &self.state).await?;
                        self.state.effects.extend(effects.into_iter().map(|e| e.into()));
                    }
                    _ => {}
                }
            }
            ClientMessage::EndTurn { player_id, .. } => {
                self.state.effects.push_back(
                    Effect::EndTurn {
                        player_id: player_id.clone(),
                    }
                    .into(),
                );
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
            player_id: player_id.clone(),
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

        // if let Phase::PreEndTurn { player_id } = self.state.phase {
        //     // If the current player is not the one ending their turn, it means we've already
        //     // actioned the pre-end turn changes, so no action is needed.
        //     if self.state.current_player == player_id && !self.state.waiting_for_input {
        //         let current_index = self
        //             .state
        //             .players
        //             .iter()
        //             .position(|p| p.id == self.state.current_player)
        //             .unwrap_or_default();
        //         let current_player = self.state.current_player.clone();
        //         let next_player = self
        //             .state
        //             .players
        //             .iter()
        //             .cycle()
        //             .skip(current_index + 1)
        //             .next()
        //             .ok_or(anyhow::anyhow!("No next player found"))?;
        //         self.state.current_player = next_player.id.clone();
        //         self.state.turns += 1;
        //         let effects = vec![
        //             Effect::EndTurn {
        //                 player_id: current_player,
        //             },
        //             Effect::StartTurn {
        //                 player_id: next_player.id.clone(),
        //             },
        //         ];
        //         self.state.effects.extend(effects.into_iter().map(|e| e.into()));
        //     }
        // }

        // Move attached artifacts to the same zone as the unit they are attached to
        let attached_artifacts: Vec<(uuid::Uuid, uuid::Uuid)> = self
            .state
            .cards
            .iter()
            .filter(|c| c.is_artifact())
            .filter_map(
                |c| match c.get_artifact_base().expect("artifact to have a base").bearer {
                    Some(attached_to) => Some((c.get_id().clone(), attached_to.clone())),
                    None => None,
                },
            )
            .collect();
        for (artifact_id, unit_id) in attached_artifacts {
            let unit = self.state.get_card(&unit_id);
            let zone = unit.get_zone().clone();
            let artifact = self.state.get_card_mut(&artifact_id);
            artifact.set_zone(zone);
        }

        self.broadcast(&self.state.into_sync()?).await?;
        Ok(())
    }

    pub async fn broadcast(&self, message: &ServerMessage) -> anyhow::Result<()> {
        for stream in self.streams.values() {
            Client::send_to_stream(message, Arc::clone(stream)).await?;
        }
        Ok(())
    }

    pub fn draw_initial_six(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for player in &self.state.players {
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
                auras_to_dispell.push(aura.get_id().clone());
            }
        }

        for aura_id in auras_to_dispell {
            {
                let card = state.get_card_mut(&aura_id);
                card.set_zone(Zone::Cemetery);
            }

            let card = state.get_card(&aura_id);
            let effects = card.deathrite(state, card.get_zone());
            state.effects.extend(effects.into_iter().map(|e| e.into()));
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

                if let Ok(Some(description)) = effect.description(&self.state).await {
                    self.broadcast(&ServerMessage::LogEvent {
                        id: uuid::Uuid::new_v4(),
                        description,
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
                self.state.effect_log.push(effect);
                self.state.compute_world_effects().await?;

                Self::dispell_auras(&mut self.state).await?;
                self.broadcast(&self.state.into_sync()?).await?;
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
