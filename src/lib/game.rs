use crate::prelude::*;
use crate::{
    card::{Ability, AdditionalCost, Aura, CardType, Cost, Region},
    effect::{Effect, EffectEngine},
    error::GameError,
    evaluation,
    networking::{
        client::Client,
        message::{ClientMessage, ServerMessage},
    },
    query::{CardQuery, QueryCache},
    state::{Phase, PlayerWithDeck, State},
};
use async_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, iter::Sum, sync::Arc};
use tokio::{net::tcp::OwnedWriteHalf, sync::Mutex};

pub type PlayerId = uuid::Uuid;
pub type CardId = uuid::Uuid;
pub const NO_CONTROLLER: PlayerId = uuid::Uuid::nil();

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
    card_ids: &[CardId],
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    pick_card_with_options(player_id, card_ids, card_ids, false, state, prompt).await
}

pub async fn pick_card_with_options(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[CardId],
    pickable_card_ids: &[CardId],
    block_opponent: bool,
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    pick_card_with_options_source(
        player_id,
        card_ids,
        pickable_card_ids,
        block_opponent,
        state,
        prompt,
        None,
    )
    .await
}

pub async fn pick_card_with_options_source(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[CardId],
    pickable_card_ids: &[CardId],
    block_opponent: bool,
    state: &State,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<uuid::Uuid> {
    let decision_player = state.decision_player(player_id.as_ref());
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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
    attacker: &CardId,
    amount: u16,
    defenders: &[CardId],
    state: &State,
) -> anyhow::Result<HashMap<CardId, u16>> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::DistributeDamage {
            player_id: decision_player,
            attacker: *attacker,
            defenders: defenders.to_vec(),
            damage: amount,
        })
        .await?;

    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::ResolveCombat {
            damage_assignment, ..
        } => {
            let assigned: u16 = damage_assignment.values().copied().sum();
            if assigned != amount {
                return Err(anyhow::anyhow!(
                    "assigned combat damage {} does not match available damage {}",
                    assigned,
                    amount
                ));
            }
            if damage_assignment
                .keys()
                .any(|card_id| !defenders.contains(card_id))
            {
                return Err(anyhow::anyhow!(
                    "combat damage assignment contains non-defender card"
                ));
            }

            Ok(defenders
                .iter()
                .map(|defender_id| {
                    (
                        *defender_id,
                        damage_assignment.get(defender_id).copied().unwrap_or(0),
                    )
                })
                .collect())
        }
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => unreachable!(),
    }
}

pub async fn pick_cards(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[CardId],
    state: &State,
    prompt: &str,
) -> anyhow::Result<Vec<CardId>> {
    pick_cards_source(player_id, card_ids, state, prompt, None).await
}

pub async fn pick_cards_source(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[CardId],
    state: &State,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<Vec<CardId>> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::PickCards {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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
    preview_cards: &[CardId],
    state: &State,
    prompt: &str,
) -> anyhow::Result<()> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::RevealCards {
            prompt: prompt.to_string(),
            player_id: decision_player,
            cards: preview_cards.to_vec(),
            action: None,
        })
        .await?;

    Ok(())
}

pub async fn take_action(
    player_id: impl AsRef<PlayerId>,
    preview_cards: &[CardId],
    state: &State,
    prompt: &str,
    action: &str,
) -> anyhow::Result<bool> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::RevealCards {
            prompt: prompt.to_string(),
            player_id: decision_player,
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
    card_ids: &[CardId],
    state: &State,
    prompt: &str,
) -> anyhow::Result<uuid::Uuid> {
    pick_card_source(player_id, card_ids, state, prompt, None).await
}

pub async fn pick_card_source(
    player_id: impl AsRef<PlayerId>,
    card_ids: &[CardId],
    state: &State,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<uuid::Uuid> {
    let decision_player = state.decision_player(player_id.as_ref());
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    state
        .get_sender()
        .send(ServerMessage::PickCard {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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
        n => unreachable!("expected PickCard, got {:?}", n),
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
    pick_action_source(player_id, actions, state, prompt, anchor_on_cursor, None).await
}

pub async fn pick_action_source<'a>(
    player_id: impl AsRef<PlayerId>,
    actions: &'a [Box<dyn ActivatedAbility>],
    state: &State,
    prompt: &str,
    anchor_on_cursor: bool,
    source_card_id: Option<CardId>,
) -> anyhow::Result<&'a Box<dyn ActivatedAbility>> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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
    let decision_player = state.decision_player(player_id);
    state
        .get_sender()
        .send(ServerMessage::Resume {
            player_id: decision_player,
        })
        .await?;

    Ok(())
}

pub async fn wait_for_opponent(
    player_id: &PlayerId,
    state: &State,
    prompt: impl AsRef<str>,
) -> anyhow::Result<()> {
    let decision_player = state.decision_player(player_id);
    state
        .get_sender()
        .send(ServerMessage::Wait {
            player_id: decision_player,
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
    yes_or_no_source(player_id, state, prompt, None).await
}

pub async fn yes_or_no_source(
    player_id: impl AsRef<PlayerId>,
    state: &State,
    prompt: impl AsRef<str>,
    source_card_id: Option<CardId>,
) -> anyhow::Result<bool> {
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;

    let options = [BaseOption::Yes, BaseOption::No];
    let option_labels = options
        .iter()
        .map(|o| o.to_string())
        .collect::<Vec<String>>();
    let choice = pick_option_source(
        player_id,
        &option_labels,
        state,
        prompt,
        false,
        source_card_id,
    )
    .await?;

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
    pick_amount_source(player_id, min_amount, max_amount, state, prompt, None).await
}

pub async fn pick_amount_source(
    player_id: impl AsRef<PlayerId>,
    min_amount: u8,
    max_amount: u8,
    state: &State,
    prompt: impl AsRef<str>,
    source_card_id: Option<CardId>,
) -> anyhow::Result<u8> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::PickAmount {
            prompt: prompt.as_ref().to_string(),
            source_card_id,
            player_id: decision_player,
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
    pick_option_source(player_id, options, state, prompt, anchor_on_cursor, None).await
}

pub async fn pick_option_source(
    player_id: impl AsRef<PlayerId>,
    options: &[String],
    state: &State,
    prompt: impl AsRef<str>,
    anchor_on_cursor: bool,
    source_card_id: Option<CardId>,
) -> anyhow::Result<usize> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::PickAction {
            prompt: prompt.as_ref().to_string(),
            source_card_id,
            player_id: decision_player,
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
    pick_path_source(player_id, paths, state, prompt, None).await
}

pub async fn pick_path_source(
    player_id: impl AsRef<PlayerId>,
    paths: &[Vec<Zone>],
    state: &State,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<Vec<Zone>> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::PickPath {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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
    pick_zone_group_source(player_id, groups, state, block_opponent, prompt, None).await
}

pub async fn pick_zone_group_source(
    player_id: impl AsRef<PlayerId>,
    groups: &[Vec<Zone>],
    state: &State,
    block_opponent: bool,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<Vec<Zone>> {
    let decision_player = state.decision_player(player_id.as_ref());
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZoneGroup {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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
    pick_zone_near_source(player_id, zone, state, block_opponent, prompt, None).await
}

pub async fn pick_zone_near_source(
    player_id: impl AsRef<PlayerId>,
    zone: &Zone,
    state: &State,
    block_opponent: bool,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<Zone> {
    let decision_player = state.decision_player(player_id.as_ref());
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZone {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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
    pick_zone_source(player_id, zones, state, block_opponent, prompt, None).await
}

pub async fn pick_zone_source(
    player_id: impl AsRef<PlayerId>,
    zones: &[Zone],
    state: &State,
    block_opponent: bool,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<Zone> {
    let decision_player = state.decision_player(player_id.as_ref());
    let opponent_id = state.get_opponent_id(player_id.as_ref())?;
    if block_opponent {
        wait_for_opponent(&opponent_id, state, "Wait for opponent...").await?;
    }

    state
        .get_sender()
        .send(ServerMessage::PickZone {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
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

pub async fn force_sync_all(state: &State) -> anyhow::Result<()> {
    for player in &state.players {
        crate::game::force_sync(&player.id, state).await?;
    }

    Ok(())
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
            turn_player,
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
                    turn_player,
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
    pick_direction_source(player_id, directions, state, prompt, None).await
}

pub async fn pick_direction_source(
    player_id: impl AsRef<PlayerId>,
    directions: &[Direction],
    state: &State,
    prompt: &str,
    source_card_id: Option<CardId>,
) -> anyhow::Result<Direction> {
    let decision_player = state.decision_player(player_id.as_ref());
    state
        .get_sender()
        .send(ServerMessage::PickDirection {
            prompt: prompt.to_string(),
            source_card_id,
            player_id: decision_player,
            directions: directions.to_vec(),
        })
        .await?;

    let board_flipped = &state.player_one != player_id.as_ref();
    let msg = state.get_receiver().recv().await?;
    match msg {
        ClientMessage::PickDirection { direction, .. } => Ok(direction.normalise(board_flipped)),
        ClientMessage::PlayerDisconnected { player_id, .. } => {
            Err(GameError::PlayerDisconnected(player_id).into())
        }
        _ => panic!("expected PickDirection, got {:?}", msg),
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
    let region = match zone {
        Zone::Location(Location::Square(_, r)) | Zone::Location(Location::Intersection(_, r)) => {
            r.clone()
        }
        _ => Region::Surface,
    };
    match zone {
        Zone::Location(Location::Square(square, _)) => {
            let diagonals = match square % 5 {
                0 => vec![
                    Zone::Location(Location::Square(square.saturating_add(4), region.clone())),
                    Zone::Location(Location::Square(square.saturating_sub(6), region.clone())),
                ],
                1 => vec![
                    Zone::Location(Location::Square(square.saturating_sub(4), region.clone())),
                    Zone::Location(Location::Square(square.saturating_add(6), region.clone())),
                ],
                _ => vec![
                    Zone::Location(Location::Square(square.saturating_sub(4), region.clone())),
                    Zone::Location(Location::Square(square.saturating_add(6), region.clone())),
                    Zone::Location(Location::Square(square.saturating_add(4), region.clone())),
                    Zone::Location(Location::Square(square.saturating_sub(6), region.clone())),
                ],
            };
            adjacent.extend(diagonals);
            adjacent.retain(|s| s.get_square().unwrap_or(0) > 0);
            adjacent.retain(|s| s.get_square().unwrap_or(0) <= 20);
            adjacent.dedup();
            adjacent
        }
        Zone::Location(Location::Intersection(sqs, _)) => {
            // Nearby zones are all adjacent zones of all squares in the intersection, plus all intersections that share at least one square
            let mut nearby = Vec::new();
            for sq in sqs {
                let realm_zone = Zone::Location(Location::Square(*sq, region.clone()));
                nearby.extend(get_adjacent_zones(&realm_zone));
                // Add diagonals for each square
                let diagonals = match sq % 5 {
                    0 => vec![
                        Zone::Location(Location::Square(sq.saturating_add(4), region.clone())),
                        Zone::Location(Location::Square(sq.saturating_sub(6), region.clone())),
                    ],
                    1 => vec![
                        Zone::Location(Location::Square(sq.saturating_sub(4), region.clone())),
                        Zone::Location(Location::Square(sq.saturating_add(6), region.clone())),
                    ],
                    _ => vec![
                        Zone::Location(Location::Square(sq.saturating_sub(4), region.clone())),
                        Zone::Location(Location::Square(sq.saturating_add(6), region.clone())),
                        Zone::Location(Location::Square(sq.saturating_add(4), region.clone())),
                        Zone::Location(Location::Square(sq.saturating_sub(6), region.clone())),
                    ],
                };
                nearby.extend(diagonals);
            }
            // Add intersections that share at least one square (excluding self)
            for intersection in Zone::all_intersections() {
                if let Zone::Location(Location::Intersection(isqs, _)) = &intersection
                    && isqs != sqs
                    && isqs.iter().any(|sq| sqs.contains(sq))
                {
                    nearby.push(Zone::Location(Location::Intersection(
                        isqs.clone(),
                        region.clone(),
                    )));
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
    let region = match zone {
        Zone::Location(Location::Square(_, r)) | Zone::Location(Location::Intersection(_, r)) => {
            r.clone()
        }
        _ => Region::Surface,
    };
    match zone {
        &Zone::Location(Location::Square(square, _)) => {
            let mut adjacent = match square % 5 {
                0 => vec![
                    Zone::Location(Location::Square(square.saturating_add(5), region.clone())),
                    Zone::Location(Location::Square(square.saturating_sub(5), region.clone())),
                    Zone::Location(Location::Square(square.saturating_sub(1), region.clone())),
                    Zone::Location(Location::Square(square, region.clone())),
                ],
                1 => vec![
                    Zone::Location(Location::Square(square.saturating_add(5), region.clone())),
                    Zone::Location(Location::Square(square.saturating_sub(5), region.clone())),
                    Zone::Location(Location::Square(square.saturating_add(1), region.clone())),
                    Zone::Location(Location::Square(square, region.clone())),
                ],
                _ => vec![
                    Zone::Location(Location::Square(square.saturating_add(5), region.clone())),
                    Zone::Location(Location::Square(square.saturating_sub(5), region.clone())),
                    Zone::Location(Location::Square(square.saturating_add(1), region.clone())),
                    Zone::Location(Location::Square(square.saturating_sub(1), region.clone())),
                    Zone::Location(Location::Square(square, region.clone())),
                ],
            };
            adjacent.retain(|s| s.get_square().unwrap_or(0) <= 20);
            adjacent.retain(|s| s.get_square().unwrap_or(0) > 0);
            adjacent
        }
        Zone::Location(Location::Intersection(locs, _)) => {
            let mut locs = locs.clone();
            locs.sort();
            let mut intersections = vec![
                Zone::Location(Location::Intersection(
                    locs.iter().map(|l| l.saturating_add(5)).collect(),
                    region.clone(),
                )),
                Zone::Location(Location::Intersection(
                    locs.iter().map(|l| l.saturating_add(1)).collect(),
                    region.clone(),
                )),
            ];

            if locs[0] > 1 {
                intersections.push(Zone::Location(Location::Intersection(
                    locs.iter().map(|l| l.saturating_sub(1)).collect(),
                    region.clone(),
                )));
            }

            if locs[0] > 5 {
                intersections.push(Zone::Location(Location::Intersection(
                    locs.iter().map(|l| l.saturating_sub(5)).collect(),
                    region.clone(),
                )));
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
        .map(|(c, r)| Zone::Location(Location::Square(((r - 1) * 5 + c) as u8, Region::Surface)))
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
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>>;

    fn can_activate(
        &self,
        _card_id: &CardId,
        _player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn get_cost(&self, _card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::ZERO.clone())
    }

    /// Returns true if activating this ability counts as a "special ability interaction"
    /// for purposes of rules such as Stealth (which is lost when a special ability is used).
    /// Basic unit actions (Move, Attack, Burrow, etc.) and Cancel are not special abilities.
    fn is_special_ability(&self) -> bool {
        true
    }
}

impl Clone for Box<dyn ActivatedAbility> {
    fn clone(&self) -> Box<dyn ActivatedAbility> {
        self.clone_boxed_action()
    }
}

struct PlayableHandCard {
    player_id: PlayerId,
    card_id: CardId,
    spellcaster_id: CardId,
    card_type: CardType,
    zones: Vec<Zone>,
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
        _card_id: &CardId,
        _player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn is_special_ability(&self) -> bool {
        false
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
            BaseAction::DrawSite => Ok(vec![Effect::DrawCard {
                player_id: *player_id,
                count: 1,
                kind: DrawKind::Site,
            }]),
            BaseAction::DrawSpell => Ok(vec![Effect::DrawCard {
                player_id: *player_id,
                count: 1,
                kind: DrawKind::Spell,
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

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        match self {
            AvatarAction::PlaySite | AvatarAction::DrawSite => {
                Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
            }
        }
    }

    async fn on_select(
        &self,
        _card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            AvatarAction::PlaySite => {
                let cards: Vec<CardId> = state
                    .cards
                    .values()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone() == &Zone::Hand)
                    .filter(|c| c.get_owner_id() == player_id)
                    .map(|c| *c.get_id())
                    .collect();
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let picked_card = state.get_card(&picked_card_id);
                let avatar_id = state.get_player_avatar_id(player_id)?;
                // we pass avatar_id as the caster just to comply with the required parameters, but
                // no caster_id is actually needed here, since sites don't need one.
                let zones = picked_card.get_valid_play_zones(state, player_id, &avatar_id)?;
                let prompt = "Pick a zone to play the site";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                Ok(vec![Effect::PlayCard {
                    player_id: *player_id,
                    card_id: picked_card_id,
                    zone: zone.clone().into(),
                    spellcaster: avatar_id,
                }])
            }
            AvatarAction::DrawSite => Ok(vec![Effect::DrawCard {
                player_id: *player_id,
                count: 1,
                kind: DrawKind::Site,
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
        artifact_id: CardId,
        artifact_name: String,
    },
    DropArtifact {
        artifact_id: CardId,
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

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
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
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            UnitAction::RangedAttack => {
                let card = state.get_card(card_id);
                let prompt = "Pick a direction for ranged strike";
                let direction = pick_direction_source(
                    player_id,
                    &CARDINAL_DIRECTIONS,
                    state,
                    prompt,
                    Some(*card_id),
                )
                .await?;
                Ok(vec![Effect::ShootProjectile {
                    id: uuid::Uuid::new_v4(),
                    range: Some(card.ranged_range(state)?.unwrap_or(1)),
                    player_id: *player_id,
                    shooter: *card_id,
                    from_zone: card.get_zone().clone(),
                    direction,
                    damage: card
                        .get_power(state)?
                        .ok_or(anyhow::anyhow!("ranged attacker has no power"))?,
                    ranged_strike: true,
                    piercing: false,
                    splash_damage: None,
                }])
            }
            UnitAction::Attack => {
                let attacker = state.get_card(card_id);
                let attacker_has_stealth = attacker.has_ability(state, &Ability::Stealth);
                let cards = attacker.get_valid_attack_targets(state, false);
                let prompt = "Pick a unit to attack";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let attacked = state.get_card(&picked_card_id);

                let opponent = state
                    .players
                    .iter()
                    .find(|p| &p.id != player_id)
                    .ok_or(anyhow::anyhow!("opponent not found"))?;
                // Stealth attackers cannot be defended against (codex).
                let possible_defenders = if attacker_has_stealth {
                    vec![]
                } else {
                    state.get_defenders_for_attack(card_id, &picked_card_id)
                };
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
                        let defenders =
                            pick_cards(&opponent.id, &possible_defenders, state, "Pick defenders")
                                .await?;
                        let defend_declared_effects: Vec<Effect> = defenders
                            .iter()
                            .map(|defender_id| Effect::DeclareDefender {
                                attacker_id: *card_id,
                                defender_id: *defender_id,
                            })
                            .collect();
                        match defenders.len() {
                            // If no defenders are picked, proceed with the original attack.
                            0 => {
                                return Ok(vec![Effect::Attack {
                                    attacker_id: *card_id,
                                    defender_id: picked_card_id,
                                    defending_ids: vec![],
                                    damage_assignment: None,
                                }]);
                            }
                            // If a single defender is picked, change the attack to target the
                            // defender.
                            1 => {
                                let defender_id = defenders[0];
                                let defender = state.get_card(&defender_id);
                                let mut effects = vec![
                                    // Return the attack effect first so that MoveCard is applied
                                    // before attack and the attack happens on the correct zone.
                                    Effect::Attack {
                                        attacker_id: *card_id,
                                        defender_id,
                                        defending_ids: vec![],
                                        damage_assignment: None,
                                    },
                                    Effect::MoveCard {
                                        player_id: opponent.id,
                                        card_id: defender_id,
                                        from: defender
                                            .get_zone()
                                            .clone()
                                            .into_location()
                                            .expect("defender must be in a location"),
                                        to: LocationQuery::from_zone(attacked.get_zone().clone()),
                                        tap: true,
                                        through_path: None,
                                    },
                                ];
                                effects.extend(defend_declared_effects);
                                return Ok(effects);
                            }
                            _ => {
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

                                resume(&opponent.id, state).await?;
                                let mut effects = vec![Effect::Attack {
                                    attacker_id: *card_id,
                                    defender_id: picked_card_id,
                                    defending_ids: defenders,
                                    damage_assignment: Some(damage_distribution),
                                }];
                                effects.extend(defend_declared_effects);
                                return Ok(effects);
                            }
                        }
                    }
                }

                Ok(vec![Effect::Attack {
                    attacker_id: *card_id,
                    defender_id: picked_card_id,
                    defending_ids: vec![],
                    damage_assignment: None,
                }])
            }
            UnitAction::Move => {
                let card = state.get_card(card_id);
                let zones = card.get_valid_move_zones(state).await?;
                let prompt = "Pick a zone to move to";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                let paths = card.get_valid_move_paths(state, &zone).await?;
                let path = if paths.len() > 1 {
                    let prompt = "Pick a path to move along";
                    pick_path(player_id, &paths, state, prompt).await?
                } else {
                    paths
                        .first()
                        .ok_or(anyhow::anyhow!("no paths found"))?
                        .to_vec()
                };

                let opponent = state
                    .players
                    .iter()
                    .find(|p| &p.id != player_id)
                    .ok_or(anyhow::anyhow!("opponent not found"))?;
                let legal_interceptors =
                    state.get_interceptors_for_move(&path, card_id, &opponent.id);
                let mut interceptors = Vec::new();
                let mut intercept_damage_assignment = None;
                if !legal_interceptors.is_empty() {
                    let final_zone = path
                        .last()
                        .ok_or(anyhow::anyhow!("move path had no final zone"))?;

                    wait_for_opponent(
                        player_id,
                        state,
                        "Wait for opponent to choose whether to intercept".to_string(),
                    )
                    .await?;

                    let prompt = format!(
                        "Pick units at {} to intercept {}",
                        final_zone,
                        card.get_name()
                    );
                    let picked_interceptors =
                        pick_cards(&opponent.id, &legal_interceptors, state, &prompt).await?;
                    interceptors = picked_interceptors
                        .into_iter()
                        .filter(|id| legal_interceptors.contains(id))
                        .collect();

                    resume(player_id, state).await?;

                    if !interceptors.is_empty() {
                        let card_power = card
                            .get_power(state)?
                            .ok_or(anyhow::anyhow!("intercepted unit has no power"))?;
                        let damage_assignment = if interceptors.len() == 1 {
                            HashMap::from([(interceptors[0], card_power)])
                        } else {
                            wait_for_opponent(
                                &opponent.id,
                                state,
                                "Wait for opponent to distribute damage".to_string(),
                            )
                            .await?;

                            let damage_distribution = distribute_damage(
                                player_id,
                                card_id,
                                card_power,
                                &interceptors,
                                state,
                            )
                            .await?;

                            resume(&opponent.id, state).await?;
                            damage_distribution
                        };

                        intercept_damage_assignment = Some(damage_assignment);
                    }
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
                        from: zone
                            .clone()
                            .into_location()
                            .expect("move path step must be a location"),
                        to: LocationQuery::from_zone(to_zone.clone()),
                        tap: true,
                        through_path: Some(path.clone()),
                    });
                }

                if let Some(damage_assignment) = intercept_damage_assignment {
                    effects.push(Effect::Attack {
                        attacker_id: *card_id,
                        defender_id: interceptors[0],
                        defending_ids: interceptors,
                        damage_assignment: Some(damage_assignment),
                    });
                }

                effects.reverse();
                Ok(effects)
            }
            UnitAction::Burrow => Ok(vec![Effect::SetCardRegion {
                card_id: *card_id,
                destination: Region::Underground,
                tap: true,
            }]),
            UnitAction::Submerge => Ok(vec![Effect::SetCardRegion {
                card_id: *card_id,
                destination: Region::Underwater,
                tap: true,
            }]),
            UnitAction::Surface => Ok(vec![Effect::SetCardRegion {
                card_id: *card_id,
                destination: Region::Surface,
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
                let minions: Vec<CardId> = CardQuery::new()
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
                    .values()
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

    fn is_special_ability(&self) -> bool {
        // All UnitAction variants are basic unit actions, not special card abilities.
        false
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
        self.state.queue(self.place_avatars());
        self.state.queue(self.draw_initial_six());

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

    fn playable_hand_card(
        &self,
        player_id: &PlayerId,
        card_id: &CardId,
    ) -> anyhow::Result<Option<PlayableHandCard>> {
        if player_id != &self.state.current_turn_controller() {
            return Ok(None);
        }

        let acting_player = self.state.current_player();
        let card = self.state.get_card(card_id);
        if !card.is_playable(&self.state, &acting_player)? {
            return Ok(None);
        }

        let avatar_id = self.state.get_player_avatar_id(&acting_player)?;
        let card_type = card.get_card_type();
        let zones = match card_type {
            CardType::Site => {
                let avatar = self.state.get_card(&avatar_id);
                let Some(avatar) = avatar.get_avatar() else {
                    return Ok(None);
                };
                let Some(action) = avatar.get_play_site_ability() else {
                    return Ok(None);
                };
                let cost = action.get_cost(&avatar_id, &self.state)?;
                if !cost.can_afford(&self.state, acting_player)? {
                    return Ok(None);
                }

                card.get_valid_play_zones(&self.state, &acting_player, &avatar_id)?
            }
            CardType::Artifact | CardType::Minion | CardType::Aura => {
                let avatar = self.state.get_card(&avatar_id);
                if !avatar
                    .can_cast_spell_with_id(&self.state, card_id, &acting_player)
                    .unwrap_or_default()
                {
                    return Ok(None);
                }

                card.get_valid_play_zones(&self.state, &acting_player, &avatar_id)?
                    .into_iter()
                    .filter(|zone| {
                        self.state
                            .get_effective_costs(card_id, Some(zone), &acting_player)
                            .and_then(|cost| cost.can_afford(&self.state, acting_player))
                            .unwrap_or(false)
                    })
                    .collect()
            }
            CardType::Magic | CardType::Avatar => Vec::new(),
        };

        if zones.is_empty() {
            return Ok(None);
        }

        Ok(Some(PlayableHandCard {
            player_id: acting_player,
            card_id: *card_id,
            spellcaster_id: avatar_id,
            card_type,
            zones,
        }))
    }

    async fn queue_play_hand_card_at_zone(
        &mut self,
        player_id: &PlayerId,
        card_id: &CardId,
        zone: &Zone,
    ) -> anyhow::Result<()> {
        let Some(playable) = self.playable_hand_card(player_id, card_id)? else {
            return Ok(());
        };
        if !playable.zones.contains(zone) {
            return Ok(());
        }

        if playable.card_type == CardType::Site {
            let avatar = self.state.get_card(&playable.spellcaster_id);
            let Some(avatar) = avatar.get_avatar() else {
                return Ok(());
            };
            let Some(action) = avatar.get_play_site_ability() else {
                return Ok(());
            };
            if !action.can_activate(&playable.spellcaster_id, &playable.player_id, &self.state)? {
                return Ok(());
            }
            let cost = action.get_cost(&playable.spellcaster_id, &self.state)?;
            if !cost.can_afford(&self.state, playable.player_id)? {
                return Ok(());
            }
            cost.pay(&mut self.state, &playable.player_id).await?;
            let effects = self
                .state
                .get_card(&playable.spellcaster_id)
                .get_avatar()
                .ok_or(anyhow::anyhow!("play site card must be an avatar"))?
                .play_site_at_zone(&self.state, &playable.player_id, &playable.card_id, zone)
                .await?;
            self.state.queue(effects);
        } else {
            self.state.queue_one(Effect::PlayCard {
                player_id: playable.player_id,
                card_id: playable.card_id,
                zone: zone.clone().into(),
                spellcaster: playable.spellcaster_id,
            });
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: &ClientMessage) -> anyhow::Result<()> {
        self.state.validate_client_message(message)?;
        match message {
            ClientMessage::PlayerDisconnected { player_id, .. } => {
                self.player_disconnected(player_id).await?;
            }
            ClientMessage::RequestPlayableZones {
                player_id, card_id, ..
            } => {
                if let Some(playable) = self.playable_hand_card(player_id, card_id)? {
                    self.state
                        .get_sender()
                        .send(ServerMessage::PlayableZones {
                            player_id: playable.player_id,
                            card_id: playable.card_id,
                            zones: playable.zones,
                        })
                        .await?;
                }
            }
            ClientMessage::RequestAuraAffectedZones {
                player_id, card_id, ..
            } => {
                let zones = self
                    .state
                    .get_card(card_id)
                    .get_aura()
                    .map(|aura| aura.get_affected_zones(&self.state))
                    .unwrap_or_default();

                self.state
                    .get_sender()
                    .send(ServerMessage::AuraAffectedZones {
                        player_id: *player_id,
                        card_id: *card_id,
                        zones,
                    })
                    .await?;
            }
            ClientMessage::RequestOngoingEffects { player_id, .. } => {
                self.state
                    .get_sender()
                    .send(ServerMessage::OngoingEffects {
                        player_id: *player_id,
                        effects: self.state.ongoing_effects_data(),
                    })
                    .await?;
            }
            ClientMessage::PlayCardAtZone {
                player_id,
                card_id,
                zone,
                ..
            } => {
                self.queue_play_hand_card_at_zone(player_id, card_id, zone)
                    .await?;
            }
            ClientMessage::ClickCard {
                player_id, card_id, ..
            } => {
                let snapshot = self.state.clone();
                let card = snapshot.get_card(card_id);
                if player_id != &self.state.current_turn_controller() {
                    return Ok(());
                }

                let acting_player = self.state.current_player();
                if card.get_zone().is_in_play() {
                    if card.get_controller_id(&self.state) != acting_player {
                        return Ok(());
                    }

                    let unit_disabled =
                        card.has_status(&self.state, &CardStatus::SummoningSickness);
                    if self.state.is_unit_card(card.get_id()) && unit_disabled {
                        return Ok(());
                    }

                    let mut actions = card.get_activated_abilities(&self.state)?;
                    actions.retain(|action| {
                        let can_afford = action
                            .get_cost(card_id, &self.state)
                            .and_then(|cost| cost.can_afford(&self.state, acting_player))
                            .unwrap_or_default();
                        let can_activate = action
                            .can_activate(card_id, &acting_player, &self.state)
                            .unwrap_or_default();
                        can_afford && can_activate
                    });

                    if actions.is_empty() {
                        return Ok(());
                    }

                    actions.push(Box::new(CancelAction));
                    let prompt = format!("{}: Pick action", card.get_name());
                    let action =
                        pick_action(&acting_player, &actions, &self.state, &prompt, true).await?;
                    let cost = action.get_cost(card_id, &self.state)?.clone();
                    let effects = action
                        .on_select(card.get_id(), &acting_player, &self.state)
                        .await?;
                    // Pay costs after selecting the action to work around scenarios where
                    // the cost includes sacricing the card and the effects involve nearby
                    // cards, which would result in no valid targets, as the card would already
                    // be in the cemetery at the point of execution the action.
                    cost.pay(&mut self.state, &acting_player).await?;
                    // Activating a special ability is an interaction: the card loses Stealth.
                    if action.is_special_ability() {
                        self.state
                            .get_card_mut(card_id)
                            .remove_modifier(&Ability::Stealth);
                    }
                    self.state.queue(effects);
                    return Ok(());
                }

                if !card.is_playable(&self.state, &acting_player)? {
                    return Ok(());
                }

                let avatar_id = self.state.get_player_avatar_id(&acting_player)?;
                if card.get_card_type() == CardType::Site {
                    let Some(playable) = self.playable_hand_card(player_id, card_id)? else {
                        return Ok(());
                    };

                    let prompt = "Pick a zone to play the site";
                    let zone =
                        pick_zone(&acting_player, &playable.zones, &self.state, false, prompt)
                            .await?;
                    self.queue_play_hand_card_at_zone(player_id, card_id, &zone)
                        .await?;
                    return Ok(());
                }

                let spellcasters = CardQuery::new().units().in_play().all(&self.state);
                let spellcasters = spellcasters
                    .into_iter()
                    .filter(|c| {
                        let can_cast = self
                            .state
                            .get_card(c)
                            .can_cast_spell_with_id(&self.state, card_id, &acting_player)
                            .unwrap_or_default();
                        let can_afford = card
                            .is_affordable(&self.state, &acting_player, &avatar_id)
                            .unwrap_or_default();
                        can_cast && can_afford
                    })
                    .collect::<Vec<_>>();

                if spellcasters.is_empty() {
                    return Ok(());
                }

                match card.get_card_type() {
                    CardType::Artifact | CardType::Minion | CardType::Aura => {
                        let mut caster_id = avatar_id;
                        if card.get_base().needs_explicit_spellcaster && spellcasters.len() > 1 {
                            let prompt = "Pick a spellcaster to cast the spell";
                            caster_id =
                                pick_card(&acting_player, &spellcasters, &self.state, prompt)
                                    .await?;
                        }

                        let effects = card
                            .play_mechanic(&self.state, &acting_player, &caster_id)
                            .await?;
                        self.state.queue(effects);
                    }
                    CardType::Magic => {
                        let mut caster_id = avatar_id;
                        if spellcasters.len() > 1 {
                            let prompt = "Pick a spellcaster to cast the spell";
                            caster_id =
                                pick_card(&acting_player, &spellcasters, &self.state, prompt)
                                    .await?;
                        }

                        let caster = self.state.get_card(&caster_id);
                        self.state.queue_one(Effect::PlayMagic {
                            player_id: acting_player,
                            card_id: *card_id,
                            caster_id,
                            from: caster
                                .get_zone()
                                .clone()
                                .into_location()
                                .expect("spell caster must be in a location"),
                        });
                    }
                    // Sites are not playable by clicking, they have to be played through avatar
                    // actions, so ignore clicks on them in the hand.
                    CardType::Site => {}
                    // Avatars should not even be playable from any zone.
                    CardType::Avatar => {}
                }
            }
            ClientMessage::EndTurn { player_id, .. } => {
                if player_id != &self.state.current_turn_controller() {
                    return Ok(());
                }

                self.state.queue_one(Effect::EndTurn {
                    player_id: self.state.current_player(),
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
                    Effect::DrawCard {
                        player_id: *player_id,
                        count: site_count as u8,
                        kind: DrawKind::Site,
                    },
                    Effect::DrawCard {
                        player_id: *player_id,
                        count: spell_count as u8,
                        kind: DrawKind::Spell,
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

    pub fn end_game(&mut self) -> anyhow::Result<()> {
        QueryCache::clear_game_cache(&self.id);
        Ok(())
    }

    pub async fn player_disconnected(&mut self, player_id: &PlayerId) -> anyhow::Result<()> {
        self.streams.retain(|pid, _| pid != player_id);
        self.broadcast(&ServerMessage::PlayerDisconnected {
            player_id: *player_id,
        })
        .await?;
        self.end_game()?;

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
            .values()
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

    pub async fn send_to_player(&self, message: &ServerMessage) -> anyhow::Result<()> {
        let player_id = message.player_id();
        let stream = self
            .streams
            .get(&player_id)
            .ok_or(anyhow::anyhow!("failed to get stream for player"))?;
        Client::send_to_stream(message, Arc::clone(stream)).await
    }

    /// Build a `Sync` message for the current state.  When `debug_eval` is
    /// enabled the message also carries a full board evaluation.
    pub(crate) fn make_sync(&self) -> anyhow::Result<ServerMessage> {
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

    pub(crate) fn game_over_messages(&self) -> Option<Vec<ServerMessage>> {
        let winner = self.state.winner_if_game_over()?;

        Some(
            self.state
                .players
                .iter()
                .map(|player| ServerMessage::GameOver {
                    player_id: player.id,
                    winner_id: winner.id,
                    winner_name: winner.name.clone(),
                })
                .collect(),
        )
    }

    pub fn draw_initial_six(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for player in &self.state.players {
            effects.push(Effect::DrawCard {
                player_id: player.id,
                count: 3,
                kind: DrawKind::Site,
            });

            effects.push(Effect::DrawCard {
                player_id: player.id,
                count: 3,
                kind: DrawKind::Spell,
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

            effects.push(Effect::SetCardZone {
                card_id: avatar_id,
                zone: Zone::Location(Location::Square(square, Region::Surface)),
            });
        }
        effects
    }

    pub(crate) async fn dispell_auras(state: &mut State) -> anyhow::Result<()> {
        let auras: Vec<&dyn Aura> = state.cards.values().filter_map(|c| c.get_aura()).collect();
        let mut auras_to_dispell = vec![];
        for aura in auras {
            if aura.should_dispell(state)? {
                auras_to_dispell.push(*aura.get_id());
            }
        }

        for aura_id in auras_to_dispell {
            state.queue_one(Effect::BuryCard { card_id: aura_id });
        }

        Ok(())
    }

    pub async fn process_effects(&mut self) -> anyhow::Result<()> {
        EffectEngine::drain_with_log(self).await
    }
}
