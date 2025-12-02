use crate::{
    card::{
        spell::{Ability, SpellBase, SpellType},
        CardBase, CardType, CardZone, Edition,
    },
    effect::{Action, Effect, GameAction, PlayerAction},
    game::{Phase, State},
    networking::Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdeptIllusionist {
    pub spell: SpellBase,
}

impl AdeptIllusionist {
    pub fn new(owner_id: uuid::Uuid, zone: CardZone) -> Self {
        Self {
            spell: SpellBase {
                card_base: CardBase {
                    id: uuid::Uuid::new_v4(),
                    owner_id,
                    zone,
                    tapped: false,
                    edition: Edition::Beta,
                },
                damage_taken: 0,
                mana_cost: 3,
                thresholds: Thresholds::parse("W"),
                power: Some(2),
                toughness: Some(2),
            },
        }
    }

    pub fn get_spell_type(&self) -> &SpellType {
        &SpellType::Minion
    }

    pub fn get_edition(&self) -> &Edition {
        &Edition::Beta
    }

    pub fn get_type(&self) -> CardType {
        CardType::Spell
    }

    pub fn get_toughness(&self) -> Option<u8> {
        self.spell.toughness
    }

    pub fn get_power(&self) -> Option<u8> {
        self.spell.power
    }

    pub fn get_abilities(&self) -> Vec<Ability> {
        vec![]
    }

    pub fn get_spell_base(&self) -> &SpellBase {
        &self.spell
    }

    pub fn get_spell_base_mut(&mut self) -> &mut SpellBase {
        &mut self.spell
    }

    pub fn get_cell_id(&self) -> Option<u8> {
        match self.spell.card_base.zone {
            CardZone::Realm(cell_id) => Some(cell_id),
            _ => None,
        }
    }

    pub fn get_name(&self) -> &str {
        "Adept Illusionist"
    }

    pub fn on_damage_taken(&self, from: &uuid::Uuid, _amount: u8, state: &State) -> Vec<Effect> {
        vec![]
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.owner_id
    }

    pub fn get_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.id
    }

    pub fn on_select_in_realm_actions(&self, state: &State) -> Vec<Action> {
        let spellbook = state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == &self.spell.card_base.owner_id)
            .filter(|c| matches!(c.get_zone(), CardZone::Spellbook))
            .filter(|c| c.get_name() == self.get_name())
            .map(|c| c.get_id())
            .cloned()
            .collect();
        let cemetery = state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| matches!(c.get_zone(), CardZone::Cemetery))
            .filter(|c| c.get_name() == self.get_name())
            .map(|c| c.get_id())
            .cloned()
            .collect();
        let hand = state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| matches!(c.get_zone(), CardZone::Hand))
            .filter(|c| c.get_name() == self.get_name())
            .map(|c| c.get_id())
            .cloned()
            .collect();

        vec![Action::PlayerAction(PlayerAction::ActivateTapAbility {
            after_select: vec![
                Effect::TapCard {
                    card_id: self.get_id().clone(),
                },
                Effect::ChangePhase {
                    new_phase: Phase::SelectingCardOutsideRealm {
                        player_id: self.get_owner_id().clone(),
                        spellbook: Some(spellbook),
                        cemetery: Some(cemetery),
                        hand: Some(hand),
                        owner: Some(self.get_owner_id().clone()),
                        after_select: Some(Action::GameAction(GameAction::SummonMinion {
                            card_id: self.get_id().clone(),
                        })),
                    },
                },
            ],
        })]
    }
}
