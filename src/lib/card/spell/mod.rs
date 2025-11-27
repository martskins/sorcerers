use crate::{
    card::{CardBase, CardType, CardZone, Target},
    effect::{Action, Effect, GameAction},
    game::{Phase, Resources, State},
    networking::Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuraType {
    #[default]
    None,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MagicType {
    #[default]
    None,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactType {
    #[default]
    None,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MinionType {
    #[default]
    None,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellType {
    #[default]
    None,
    Minion(MinionType),
    Artifact(ArtifactType),
    Magic(MagicType),
    Aura(AuraType),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellBase {
    pub card_base: CardBase,
    pub power: Option<u8>,
    // TODO: Implement damange reset at the end of turn
    pub damage_taken: u8,
    pub spell_type: SpellType,
}

/// Represents the different spell cards in the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Spell {
    BurningHands(SpellBase),
    BallLightning(SpellBase),
}

impl Spell {
    /// Returns a reference to the underlying `CardBase` of the spell.
    pub fn get_spell_base(&self) -> &SpellBase {
        match self {
            Spell::BurningHands(cb) => &cb,
            Spell::BallLightning(cb) => &cb,
        }
    }

    /// Returns a reference to the underlying `CardBase` of the spell.
    pub fn get_base(&self) -> &CardBase {
        match self {
            Spell::BurningHands(cb) => &cb.card_base,
            Spell::BallLightning(cb) => &cb.card_base,
        }
    }

    /// Returns a mutable reference to the underlying `CardBase` of the spell.
    pub fn get_base_mut(&mut self) -> &mut CardBase {
        match self {
            Spell::BurningHands(cb) => &mut cb.card_base,
            Spell::BallLightning(cb) => &mut cb.card_base,
        }
    }

    /// Returns a reference to the unique identifier (`Uuid`) of the spell card.
    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            Spell::BurningHands(cb) => &cb.card_base.id,
            Spell::BallLightning(cb) => &cb.card_base.id,
        }
    }

    /// Returns the name of the spell as a string slice.
    pub fn get_name(&self) -> &str {
        match self {
            Spell::BurningHands(_) => "Burning Hands",
            Spell::BallLightning(_) => "Ball Lightning",
        }
    }

    /// Returns a reference to the owner's unique identifier (`Uuid`).
    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Spell::BurningHands(cb) => &cb.card_base.owner_id,
            Spell::BallLightning(cb) => &cb.card_base.owner_id,
        }
    }

    /// Returns a reference to the current zone of the spell card.
    pub fn get_zone(&self) -> &CardZone {
        match self {
            Spell::BurningHands(cb) => &cb.card_base.zone,
            Spell::BallLightning(cb) => &cb.card_base.zone,
        }
    }

    /// Sets the zone of the spell card to a new value.
    pub fn set_zone(&mut self, new_zone: CardZone) {
        match self {
            Spell::BurningHands(cb) => cb.card_base.zone = new_zone,
            Spell::BallLightning(cb) => cb.card_base.zone = new_zone,
        };
    }

    /// Returns the effects that occur when the spell is created (genesis).
    pub fn genesis(&self) -> Vec<Effect> {
        vec![]
    }

    /// Returns the effects that occur at the start of a turn for this spell.
    pub fn on_turn_start(&self, _: &State) -> Vec<Effect> {
        vec![Effect::UntapCard {
            card_id: self.get_id().clone(),
        }]
    }

    pub fn is_permanent(&self) -> bool {
        match &self.get_spell_base().spell_type {
            SpellType::None => false,
            SpellType::Minion(_) => true,
            SpellType::Artifact(_) => true,
            SpellType::Magic(_) => false,
            SpellType::Aura(_) => true,
        }
    }

    pub fn get_power(&self) -> Option<u8> {
        match self {
            Spell::BurningHands(cb) => cb.power,
            Spell::BallLightning(cb) => cb.power,
        }
    }

    pub fn is_dead(&self) -> bool {
        let base = self.get_spell_base();
        match self.get_power() {
            None => false,
            Some(power) => base.damage_taken >= power,
        }
    }

    pub fn take_damage(&mut self, amount: u8) {
        match self {
            Spell::BurningHands(cb) => cb.damage_taken += amount,
            Spell::BallLightning(cb) => cb.damage_taken += amount,
        }
    }

    pub fn reset_damage(&mut self) {
        match self {
            Spell::BurningHands(cb) => cb.damage_taken = 0,
            Spell::BallLightning(cb) => cb.damage_taken = 0,
        }
    }

    /// Returns the effects that occur when the spell is selected, given the current game state.
    /// It also does basic state checks like verifying if the owner has enough mana to trigger any
    /// actions on the card or not.
    pub fn on_select(&self, state: &State) -> Vec<Effect> {
        match self.get_zone() {
            CardZone::None => unreachable!(),
            CardZone::Hand => self.on_select_in_hand(state),
            CardZone::Spellbook => todo!(),
            CardZone::Atlasbook => todo!(),
            CardZone::DiscardPile => todo!(),
            CardZone::Realm(_) => self.on_select_in_realm(state),
        }
    }

    fn on_select_in_realm(&self, _state: &State) -> Vec<Effect> {
        if !self.is_permanent() {
            return vec![];
        }

        match self {
            Spell::BurningHands(_) => vec![],
            Spell::BallLightning(_) => vec![],
        }
    }

    fn on_select_in_hand(&self, state: &State) -> Vec<Effect> {
        let owner_id = self.get_owner_id();
        let resources = state.resources.get(&owner_id).cloned().unwrap_or(Resources::new());
        if !resources.has_enough_for_spell(self) {
            return vec![];
        }

        match self {
            Spell::BurningHands(_) => vec![
                Effect::SetTargeting(1),
                Effect::ChangePhase {
                    new_phase: Phase::SelectingCard {
                        player_id: self.get_owner_id().clone(),
                        card_ids: state
                            .cards
                            .iter()
                            .filter(|c| matches!(c.get_zone(), CardZone::Realm(_)))
                            .filter(|c| c.get_type() == CardType::Spell || c.get_type() == CardType::Avatar)
                            .map(|c| c.get_id())
                            .cloned()
                            .collect(),
                        amount: 1,
                        after_select: Some(Action::GameAction(GameAction::PlayCardOnSelectedTargets {
                            card_id: self.get_id().clone(),
                        })),
                    },
                },
            ],
            Spell::BallLightning(_) => vec![
                Effect::SetTargeting(1),
                Effect::ChangePhase {
                    new_phase: Phase::SelectingCard {
                        player_id: self.get_owner_id().clone(),
                        card_ids: state
                            .cards
                            .iter()
                            .filter(|c| matches!(c.get_zone(), CardZone::Realm(_)))
                            .filter(|c| c.get_type() == CardType::Spell || c.get_type() == CardType::Avatar)
                            .map(|c| c.get_id())
                            .cloned()
                            .collect(),
                        amount: 1,
                        after_select: Some(Action::GameAction(GameAction::PlayCardOnSelectedTargets {
                            card_id: self.get_id().clone(),
                        })),
                    },
                },
            ],
        }
    }

    /// Returns the mana cost required to play the spell.
    pub fn get_mana_cost(&self) -> u8 {
        match self {
            Spell::BurningHands(_) => 3,
            Spell::BallLightning(_) => 2,
        }
    }

    /// Returns the required thresholds to play the spell.
    pub fn get_required_threshold(&self) -> Thresholds {
        match self {
            Spell::BurningHands(_) => Thresholds::new(1, 0, 0, 0),
            Spell::BallLightning(_) => Thresholds::new(0, 0, 0, 2),
        }
    }

    pub fn on_cast(&self, _state: &State, target: Target) -> Vec<Effect> {
        let mut effects = vec![];
        effects.push(Effect::SpendMana {
            player_id: self.get_owner_id().clone(),
            amount: self.get_mana_cost(),
        });

        match self {
            Spell::BurningHands(_) | Spell::BallLightning(_) => {
                match target {
                    Target::Card(target_id) => {
                        // TODO: Change DealDamage to support area of effect damage.
                        effects.push(Effect::DealDamage { target_id, amount: 4 });
                    }
                    _ => unreachable!(),
                }
            }
        }

        effects
    }

    /// Returns the effects that occur when the player selects a card with the intention to play
    /// it.
    pub fn on_prepare(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    /// Returns the effects that occur after the spell has been resolved.
    pub fn after_resolve(&self, _state: &State) -> Vec<Effect> {
        if !self.is_permanent() {
            return vec![Effect::MoveCard {
                card_id: self.get_id().clone(),
                to_zone: CardZone::DiscardPile,
            }];
        }

        vec![]
    }

    pub fn is_unit(&self) -> bool {
        match &self.get_spell_base().spell_type {
            SpellType::Minion(_) => true,
            _ => false,
        }
    }
}
