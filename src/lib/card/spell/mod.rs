mod beta;
use beta::*;
mod util;
use crate::{
    card::{site::SiteType, CardBase, CardType, CardZone, Edition, Element, Target, Thresholds},
    effect::{Action, Effect, GameAction, PlayerAction},
    game::{Cell, Phase, Resources, State},
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
    pub mana_cost: u8,
    pub thresholds: Thresholds,
    pub power: Option<u8>,
    pub toughness: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ability {
    Airborne,
    Charge,
    Burrowing,
    Submerge,
    Movement(u8),
    Voidwalk,
    Spellcaster(Option<Vec<Element>>),
    Lethal,
    ImmuneToSpells(Option<Vec<Element>>),
    ImmuneToSites(Option<Vec<SiteType>>),
}

#[rustfmt::skip]
spells!(
    AccursedAlbatross, "Accursed Albatross",
    AdeptIllusionist, "Adept Illusionist",
    InfernalLegion, "Infernal Legion",
    EscyllionCyclops, "Escyllion Cyclops",
    AskelonPhoenix, "Askelon Phoenix",
    SandWorm, "Sand Worm",
    PetrosianCavalry, "Petrosian Cavalry",
    HillockBasilisk, "Hillock Basilisk",
    ClamorOfHarpies, "Clamor of Harpies",
    QuarrelsomeKobolds, "Quarrelsome Kobolds",
    OgreGoons, "Ogre Goons",
    ColickyDragonettes, "Colicky Dragonettes",
    WayfaringPilgrim, "Wayfaring Pilgrim",
    SacredScarabs, "Sacred Scarabs",
    RimlandNomads, "Rimland Nomads",
    LavaSalamander, "Lava Salamander",
    RaalDromedary, "Raal Dromedary",
    PitVipers, "Pit Vipers",
    MajorExplosion, "Major Explosion",
    ConeOfFlame, "Cone of Flame",
    Incinerate, "Incinerate",
    Fireball, "Fireball",
    MinorExplosion, "Minor Explosion",
    HeatRay, "Heat Ray",
    Blaze, "Blaze",
    MadDash, "Mad Dash",
    Firebolts, "Firebolts",
    Wildfire, "Wildfire"
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
        match self.get_toughness() {
            None => false,
            Some(toughness) => base.damage_taken >= toughness,
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
            CardZone::Cemetery => todo!(),
            CardZone::Realm(_) => self.on_select_in_realm(state),
        }
    }

    pub fn get_cell_id(&self) -> Option<u8> {
        match self.get_zone() {
            CardZone::Realm(cell_id) => Some(*cell_id),
            _ => None,
        }
    }

    fn get_valid_attack_targets(&self, state: &State) -> Vec<uuid::Uuid> {
        state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() != self.get_owner_id())
            .filter(|c| matches!(c.get_zone(), CardZone::Realm(_)))
            .filter(|c| {
                let a = self.get_cell_id().unwrap();
                let b = c.get_cell_id().unwrap();
                if self.get_abilities().contains(&Ability::Airborne) {
                    return Cell::are_nearby(a, b);
                }

                Cell::are_adjacent(a, b)
            })
            .map(|c| c.get_id())
            .cloned()
            .collect::<Vec<uuid::Uuid>>()
    }

    fn valid_move_cells(&self, state: &State) -> Vec<u8> {
        state
            .cards
            .iter()
            .filter(|c| matches!(c.get_zone(), CardZone::Realm(_)))
            .filter(|c| {
                let a = self.get_cell_id().unwrap();
                let b = c.get_cell_id().unwrap();
                if self.get_abilities().contains(&Ability::Airborne) {
                    return Cell::are_nearby(a, b);
                }

                Cell::are_adjacent(a, b)
            })
            .map(|c| match c.get_zone() {
                CardZone::Realm(cell_id) => cell_id.clone(),
                _ => unreachable!(),
            })
            .collect::<Vec<u8>>()
    }

    pub fn take_damage(&self, from: &uuid::Uuid, amount: u8) -> Vec<Effect> {
        vec![Effect::DealDamage {
            target_id: *self.get_id(),
            from: from.clone(),
            amount,
        }]
    }

    pub fn on_damage_taken(&self, from: &uuid::Uuid, amount: u8, state: &State) -> Vec<Effect> {
        let effects = match self {
            Spell::AccursedAlbatross(c) => c.on_damage_taken(from, amount, state),
            Spell::AdeptIllusionist(c) => c.on_damage_taken(from, amount, state),
            _ => vec![],
        };

        effects
    }

    fn on_select_in_realm(&self, state: &State) -> Vec<Effect> {
        if !self.is_permanent() || self.get_base().tapped {
            return vec![];
        }

        let mut effects = vec![];
        let mut actions = vec![
            Action::PlayerAction(PlayerAction::Attack {
                after_select: vec![Effect::ChangePhase {
                    new_phase: Phase::SelectingCard {
                        player_id: self.get_owner_id().clone(),
                        card_ids: self.get_valid_attack_targets(state),
                        amount: 1,
                        after_select: Some(Action::GameAction(GameAction::AttackSelectedTarget {
                            attacker_id: self.get_id().clone(),
                        })),
                    },
                }],
            }),
            Action::PlayerAction(PlayerAction::Move {
                after_select: vec![Effect::ChangePhase {
                    new_phase: Phase::SelectingCell {
                        player_id: self.get_owner_id().clone(),
                        cell_ids: self.valid_move_cells(state),
                        after_select: Some(Action::GameAction(GameAction::MoveCardToSelectedCell {
                            card_id: self.get_id().clone(),
                        })),
                    },
                }],
            }),
        ];

        actions.extend(self.on_select_in_realm_actions(state));

        match self.get_spell_type() {
            SpellType::Minion => {
                effects.push(Effect::ChangePhase {
                    new_phase: Phase::SelectingAction {
                        player_id: self.get_owner_id().clone(),
                        actions,
                    },
                });
            }
            _ => {}
        }

        effects
    }

    fn on_select_in_hand(&self, state: &State) -> Vec<Effect> {
        let owner_id = self.get_owner_id();
        let resources = state.resources.get(&owner_id).cloned().unwrap_or(Resources::new());
        if !resources.has_enough_for_spell(self) {
            return vec![];
        }

        let mut effects = vec![];
        match self.get_spell_type() {
            SpellType::Minion => {
                effects.push(Effect::ChangePhase {
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
                });
            }
            SpellType::Magic => {
                effects.push(Effect::ChangePhase {
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
                });
            }
            _ => {}
        }

        effects
    }

    pub fn on_cast(&self, state: &State, target: Target) -> Vec<Effect> {
        let mut effects = vec![];
        effects.push(Effect::SpendMana {
            player_id: self.get_owner_id().clone(),
            amount: self.get_mana_cost(),
        });

        if self.is_permanent() {
            match target {
                Target::Cell(cell_id) => {
                    effects.push(Effect::MoveCard {
                        card_id: self.get_id().clone(),
                        to_zone: CardZone::Realm(cell_id),
                    });
                    effects.extend(self.genesis());
                }
                _ => unreachable!(),
            }
        }

        let card_effects = match self {
            _ => vec![],
        };
        effects.extend(card_effects);

        // match self {
        //     _ => {} // Spell::BurningHands(_) | Spell::BallLightning(_) => {
        //             //     match target {
        //             //         Target::Card(target_id) => {
        //             //             // TODO: Change DealDamage to support area of effect damage.
        //             //             effects.push(Effect::DealDamage {
        //             //                 from: self.get_id().clone(),
        //             //                 target_id,
        //             //                 amount: 4,
        //             //             });
        //             //         }
        //             //         _ => unreachable!(),
        //             //     }
        //             // }
        //             // _ => {}
        // }

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
                to_zone: CardZone::Cemetery,
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
