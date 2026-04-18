//! Board evaluation model for the Sorcery game.
//!
//! Provides [`evaluate`], which scores the current game state from each
//! player's perspective so that AI agents (and debug displays) can tell who is
//! ahead and by how much.

use crate::{
    card::{CardType, Zone},
    game::PlayerId,
    state::State,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Score weights
// ---------------------------------------------------------------------------

/// Points per remaining avatar hit-point.  Avatar death ends the game, so this
/// is weighted heavily.
const AVATAR_HEALTH_WEIGHT: f32 = 3.0;

/// Points per friendly site in play.  Sites produce mana every turn and
/// control the board, so they are extremely valuable.
const SITE_WEIGHT: f32 = 5.0;

/// Points per effective power (base + counters) of a friendly minion in play.
const MINION_POWER_WEIGHT: f32 = 1.0;

/// Points per remaining toughness (toughness − damage taken) of a friendly
/// minion in play.
const MINION_TOUGHNESS_WEIGHT: f32 = 0.5;

/// Points per card in hand.
const HAND_CARD_WEIGHT: f32 = 1.0;

/// Points awarded for each step a unit is *closer* to the enemy avatar
/// (out of a maximum distance of 8 on the 4×5 board).
const BOARD_ADVANCEMENT_WEIGHT: f32 = 0.5;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A per-player breakdown of score contributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalComponents {
    /// Contribution from the avatar's remaining health.
    pub avatar_health: f32,
    /// Contribution from sites in play.
    pub sites_in_play: f32,
    /// Contribution from the total power of friendly minions.
    pub minion_power: f32,
    /// Contribution from the total remaining toughness of friendly minions.
    pub minion_toughness: f32,
    /// Contribution from the number of cards in hand.
    pub hand_size: f32,
    /// Contribution from how close friendly units are to the enemy avatar.
    pub board_advancement: f32,
}

impl EvalComponents {
    /// Total score for this player.
    pub fn total(&self) -> f32 {
        self.avatar_health
            + self.sites_in_play
            + self.minion_power
            + self.minion_toughness
            + self.hand_size
            + self.board_advancement
    }
}

/// Snapshot evaluation of the game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    /// Total score for each player.  Higher means that player is in a stronger
    /// position.
    pub scores: HashMap<PlayerId, f32>,
    /// Detailed breakdown of score components per player.
    pub components: HashMap<PlayerId, EvalComponents>,
}

// ---------------------------------------------------------------------------
// Evaluation function
// ---------------------------------------------------------------------------

/// Evaluate the current game state and return a score for every player.
pub fn evaluate(state: &State) -> Evaluation {
    // Resolve each player's avatar zone once (needed for advancement scoring).
    let avatar_zones: HashMap<PlayerId, Zone> = state
        .players
        .iter()
        .filter_map(|p| {
            state
                .get_player_avatar_id(&p.id)
                .ok()
                .map(|id| (p.id, state.get_card(&id).get_zone().clone()))
        })
        .collect();

    let mut scores = HashMap::new();
    let mut components = HashMap::new();

    for player in &state.players {
        let pid = player.id;

        // Opponent's avatar zone (used for board-advancement scoring).
        let enemy_avatar_zone: Option<&Zone> = state
            .players
            .iter()
            .find(|p| p.id != pid)
            .and_then(|opp| avatar_zones.get(&opp.id));

        // --- Avatar health ---
        let avatar_health = score_avatar_health(state, &pid);

        // --- Sites in play ---
        let sites_in_play = state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == &pid && c.is_site() && c.get_zone().is_in_play())
            .count() as f32
            * SITE_WEIGHT;

        // --- Minion power, toughness, and board advancement ---
        let mut minion_power = 0.0f32;
        let mut minion_toughness = 0.0f32;
        let mut board_advancement = 0.0f32;

        for card in state.cards.iter().filter(|c| {
            c.get_owner_id() == &pid
                && c.get_card_type() == CardType::Minion
                && c.get_zone().is_in_play()
        }) {
            if let Some(ub) = card.get_unit_base() {
                // Use the stored power field for a cheap estimate; counter
                // effects are complex to resolve without full game context.
                let power = ub.power as f32;
                let effective_toughness = (ub.toughness as i32 - ub.damage as i32).max(0) as f32;
                minion_power += power * MINION_POWER_WEIGHT;
                minion_toughness += effective_toughness * MINION_TOUGHNESS_WEIGHT;
            }

            board_advancement += advancement_score(card.get_zone(), enemy_avatar_zone);
        }

        // Also reward the avatar for advancing toward the enemy.
        if let Some(avatar_zone) = avatar_zones.get(&pid) {
            board_advancement += advancement_score(avatar_zone, enemy_avatar_zone);
        }

        // --- Hand size ---
        let hand_size = state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == &pid && c.get_zone() == &Zone::Hand)
            .count() as f32
            * HAND_CARD_WEIGHT;

        let comp = EvalComponents {
            avatar_health,
            sites_in_play,
            minion_power,
            minion_toughness,
            hand_size,
            board_advancement,
        };
        scores.insert(pid, comp.total());
        components.insert(pid, comp);
    }

    Evaluation { scores, components }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn score_avatar_health(state: &State, player_id: &PlayerId) -> f32 {
    state
        .get_player_avatar_id(player_id)
        .ok()
        .and_then(|avatar_id| {
            let card = state.get_card(&avatar_id);
            let ub = card.get_unit_base()?;
            let hp = (ub.toughness as i32 - ub.damage as i32).max(0) as f32;
            Some(hp * AVATAR_HEALTH_WEIGHT)
        })
        .unwrap_or(0.0)
}

/// Reward a unit for being close to the enemy avatar.
/// Returns 0 if the enemy avatar zone is unknown.
fn advancement_score(unit_zone: &Zone, enemy_avatar_zone: Option<&Zone>) -> f32 {
    let enemy_zone = match enemy_avatar_zone {
        Some(z) => z,
        None => return 0.0,
    };
    if let Some(dist) = unit_zone.min_steps_to_zone(enemy_zone) {
        // On the 4×5 board the maximum possible distance is ~8 steps.
        (8u8.saturating_sub(dist)) as f32 * BOARD_ADVANCEMENT_WEIGHT
    } else {
        0.0
    }
}
