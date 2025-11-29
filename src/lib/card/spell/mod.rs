mod util;

use crate::{
    card::{CardBase, CardType, CardZone, Edition, Target},
    effect::{Action, Effect, GameAction},
    game::{Phase, Resources, State},
    networking::Thresholds,
    spells,
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
    Minion,
    Artifact,
    Magic,
    Aura,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellBase {
    pub card_base: CardBase,
    // TODO: Implement damange reset at the end of turn
    pub damage_taken: u8,
}

impl SpellBase {
    pub fn new(owner_id: uuid::Uuid, zone: CardZone) -> Self {
        Self {
            card_base: CardBase::new(owner_id, zone),
            damage_taken: 0,
        }
    }
}

#[rustfmt::skip]
spells!(
    Abundance, "Abundance", 5, "WW", SpellType::Magic, None, None, Edition::Beta,
    AccursedAlbatros, "Accursed Albatros", 3, "W", SpellType::Minion, Some(1), Some(1), Edition::Beta,
    AdeptIllusionist, "Adept Illusionist", 2, "WW", SpellType::Minion, Some(2),Some(2), Edition::Beta,
    AlbespinePikemen, "Albespine Pikemen", 3, "EE", SpellType::Minion, Some(3),Some(3), Edition::Beta,
    AllTerrainVestments, "All-Terrain Vestments", 3, "", SpellType::Artifact, None, None, Edition::Beta,
    AlvalinneDryads, "Alvalinne Dryads", 3, "", SpellType::Minion, Some(1), Some(1), Edition::Beta,
    AmazonWarriors, "Amazon Warriors", 5, "E", SpellType::Minion, Some(5),Some(5), Edition::Beta,
    AmethystCore, "Amethyst Core", 1, "", SpellType::Artifact, None, None, Edition::Beta,
    AncientDragon, "Ancient Dragon", 7, "RRR", SpellType::Minion, Some(6), Some(6), Edition::Beta,
    AngelsEgg, "Angels Egg", 3, "", SpellType::Artifact, None, None, Edition::Beta,
    AnuiUndine, "Anui Undine", 5, "WW", SpellType::Minion, Some(0),Some(0), Edition::Beta,
    ApprenticeWizard, "Apprentice Wizard", 3, "A", SpellType::Minion, Some(1),Some(1), Edition::Beta,
    AquamarineCore, "Aquamarine Core", 1, "", SpellType::Artifact, None, None, Edition::Beta,
    AramosMercenaries, "Aramos Mercenaries", 3, "FF", SpellType::Minion, Some(3),Some(3), Edition::Beta,
    AskelonPhoenix, "Askelon Phoenix", 5, "RR", SpellType::Minion, Some(4),Some(4), Edition::Beta,
    AssortedAnimas, "Assorted Animas", 0, "", SpellType::Artifact, None, None, Edition::Beta, // TODO: Implemenet X cost
    AutumnUnicon, "Autumn Unicorn", 3, "EE", SpellType::Minion, Some(4),Some(4), Edition::Beta,
    AwakenedMummies, "Awakened Mummies", 1, "F", SpellType::Minion, Some(3),Some(3), Edition::Beta,
    AzuridgeCaravan, "Azuridge Caravan", 5, "F", SpellType::Minion, Some(4),Some(4), Edition::Beta,
    BurningHands, "Burning Hands", 3, "R", SpellType::Magic, None, None, Edition::Beta,
    BallLightning, "Ball Lightning", 2, "AA", SpellType::Magic, None, None, Edition::Beta,
    BlackKnight, "Black Knight", 5, "FA", SpellType::Minion, Some(5),Some(3), Edition::Beta,
    SlyFox, "Sly Fox", 1, "W", SpellType::Minion, Some(1),Some(1), Edition::Beta,
    CastIntoExile, "Cast Into Exile", 2, "AA", SpellType::Magic, None, None, Edition::Beta
);

impl Spell {
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
        match &self.get_spell_type() {
            SpellType::None => false,
            SpellType::Minion => true,
            SpellType::Artifact => true,
            SpellType::Magic => false,
            SpellType::Aura => true,
        }
    }

    pub fn is_dead(&self) -> bool {
        let base = self.get_spell_base();
        match self.get_power() {
            None => false,
            Some(power) => base.damage_taken >= power,
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
            Spell::BlackKnight(_) => vec![],
            Spell::SlyFox(_) => vec![],
            Spell::CastIntoExile(_) => vec![],
            Spell::AdeptIllusionist(_) => vec![],
            _ => vec![],
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
            Spell::BlackKnight(_) => vec![Effect::ChangePhase {
                new_phase: Phase::SelectingCell {
                    player_id: self.get_owner_id().clone(),
                    cell_ids: state
                        .cards
                        .iter()
                        .filter(|c| c.get_owner_id() == owner_id)
                        .filter(|c| matches!(c.get_zone(), CardZone::Realm(_)))
                        .filter(|c| c.get_type() == CardType::Site)
                        .map(|c| match c.get_zone() {
                            CardZone::Realm(cell_id) => cell_id.clone(),
                            _ => unreachable!(),
                        })
                        .collect(),
                    after_select: Some(Action::GameAction(GameAction::PlayCardOnSelectedTargets {
                        card_id: self.get_id().clone(),
                    })),
                },
            }],
            Spell::SlyFox(_) => vec![],
            Spell::CastIntoExile(_) => vec![],
            Spell::AdeptIllusionist(_) => vec![],
            Spell::Abundance(_) => vec![],
            Spell::AccursedAlbatros(_) => vec![],
            Spell::AlbespinePikemen(_) => vec![],
            Spell::AllTerrainVestments(_) => vec![],
            Spell::AlvalinneDryads(_) => vec![],
            Spell::AmazonWarriors(_) => vec![],
            Spell::AmethystCore(_) => vec![],
            Spell::AncientDragon(_) => vec![],
            Spell::AngelsEgg(_) => vec![],
            _ => vec![],
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
            _ => {}
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
        match &self.get_spell_type() {
            SpellType::Minion => true,
            _ => false,
        }
    }
}
