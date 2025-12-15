use crate::{
    card::{CardType, Modifier, SiteBase, UnitBase, Zone},
    game::{PlayerId, PlayerStatus, Thresholds},
    state::State,
};
use std::fmt::Debug;

pub trait CardStatus: Debug + Send + Sync {}

#[derive(Debug, Clone)]
pub struct ModifierCounter {
    pub modifier: Modifier,
    pub expires_in_turns: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct Counter {
    pub power: i8,
    pub toughness: i8,
    pub expires_in_turns: Option<u8>,
}

impl Counter {
    pub fn new(power: i8, toughness: i8, expires_in_turns: Option<u8>) -> Self {
        Self {
            power,
            toughness,
            expires_in_turns,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Effect {
    SetPlayerStatus {
        status: PlayerStatus,
    },
    AddModifier {
        card_id: uuid::Uuid,
        counter: ModifierCounter,
    },
    AddCounter {
        card_id: uuid::Uuid,
        counter: Counter,
    },
    MoveCard {
        card_id: uuid::Uuid,
        from: Zone,
        to: Zone,
        tap: bool,
    },
    DrawCard {
        player_id: uuid::Uuid,
        card_type: CardType,
    },
    PlayMagic {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        caster_id: uuid::Uuid,
        from: Zone,
    },
    PlayCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        zone: Zone,
    },
    TapCard {
        card_id: uuid::Uuid,
    },
    EndTurn {
        player_id: uuid::Uuid,
    },
    StartTurn {
        player_id: uuid::Uuid,
    },
    RemoveResources {
        player_id: uuid::Uuid,
        mana: u8,
        thresholds: Thresholds,
        health: u8,
    },
    AddResources {
        player_id: uuid::Uuid,
        mana: u8,
        thresholds: Thresholds,
        health: u8,
    },
    Attack {
        attacker_id: uuid::Uuid,
        defender_id: uuid::Uuid,
    },
    TakeDamage {
        card_id: uuid::Uuid,
        from: uuid::Uuid,
        damage: u8,
    },
    BuryCard {
        card_id: uuid::Uuid,
    },
}

impl Effect {
    pub fn bury_card(card_id: &uuid::Uuid) -> Self {
        Effect::BuryCard {
            card_id: card_id.clone(),
        }
    }

    pub fn tap_card(card_id: &uuid::Uuid) -> Self {
        Effect::TapCard {
            card_id: card_id.clone(),
        }
    }

    pub fn wait_for_card_draw(player_id: &PlayerId) -> Self {
        Effect::SetPlayerStatus {
            status: PlayerStatus::WaitingForCardDraw {
                player_id: player_id.clone(),
            },
        }
    }

    pub fn wait_for_play(player_id: &PlayerId) -> Self {
        Effect::SetPlayerStatus {
            status: PlayerStatus::WaitingForPlay {
                player_id: player_id.clone(),
            },
        }
    }

    pub fn select_action(player_id: &PlayerId, actions: Vec<String>) -> Self {
        Effect::SetPlayerStatus {
            status: PlayerStatus::SelectingAction {
                player_id: player_id.clone(),
                actions: actions,
            },
        }
    }

    pub fn select_card(player_id: &PlayerId, valid_cards: Vec<uuid::Uuid>) -> Self {
        Effect::SetPlayerStatus {
            status: PlayerStatus::SelectingCard {
                player_id: player_id.clone(),
                valid_cards: valid_cards,
            },
        }
    }

    pub fn add_modifier(card_id: &uuid::Uuid, modifier: Modifier, expires_in_turns: Option<u8>) -> Self {
        Effect::AddModifier {
            card_id: card_id.clone(),
            counter: ModifierCounter {
                modifier,
                expires_in_turns,
            },
        }
    }

    pub fn select_square(player_id: &PlayerId, valid_squares: Vec<u8>) -> Self {
        Effect::SetPlayerStatus {
            status: PlayerStatus::SelectingSquare {
                player_id: player_id.clone(),
                valid_squares: valid_squares,
            },
        }
    }

    pub fn apply(&self, state: &mut State) -> anyhow::Result<()> {
        match self {
            Effect::MoveCard { card_id, to, tap, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(to.clone());
                let effects = card.on_move(&snapshot, to);
                state.effects.extend(effects);
                if *tap {
                    card.get_base_mut().tapped = true;
                }
            }
            Effect::DrawCard { player_id, card_type } => {
                let deck = state.decks.get_mut(player_id).unwrap();
                match card_type {
                    CardType::Site => {
                        let card_id = deck.sites.pop().unwrap();
                        state
                            .cards
                            .iter_mut()
                            .find(|c| c.get_id() == &card_id)
                            .unwrap()
                            .set_zone(Zone::Hand);
                    }
                    CardType::Spell => {
                        let card_id = deck.spells.pop().unwrap();
                        state
                            .cards
                            .iter_mut()
                            .find(|c| c.get_id() == &card_id)
                            .unwrap()
                            .set_zone(Zone::Hand);
                    }
                    CardType::Avatar => unreachable!(),
                }
            }
            Effect::SetPlayerStatus { status, .. } => {
                state.player_status = status.clone();
            }
            Effect::PlayMagic { card_id, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let mut effects = card.on_cast(&snapshot);
                let mana_cost = card.get_mana_cost(&snapshot);
                println!("Playing magic card {}, mana cost {}", card.get_name(), mana_cost);
                effects.push(Effect::RemoveResources {
                    player_id: card.get_owner_id().clone(),
                    mana: mana_cost,
                    thresholds: Thresholds::new(),
                    health: 0,
                });
                state.effects.extend(effects);
            }
            Effect::PlayCard { card_id, zone, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let cast_effects = card.on_cast(&snapshot);
                card.set_zone(zone.clone());
                if !card.has_modifier(&snapshot, Modifier::Charge) {
                    card.add_modifier(Modifier::SummoningSickness);
                }
                let mut effects = card.genesis(&snapshot);
                let mana_cost = card.get_mana_cost(&snapshot);
                effects.push(Effect::RemoveResources {
                    player_id: card.get_owner_id().clone(),
                    mana: mana_cost,
                    thresholds: Thresholds::new(),
                    health: 0,
                });
                state.effects.extend(effects);
                state.effects.extend(cast_effects);
            }
            Effect::TapCard { card_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.get_base_mut().tapped = true;
            }
            Effect::StartTurn { player_id } => {
                let cards = state
                    .cards
                    .iter_mut()
                    .filter(|c| c.get_owner_id() == &state.current_player);
                for card in cards {
                    card.get_base_mut().tapped = false;
                    card.remove_modifier(Modifier::SummoningSickness);
                }

                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana = 0;

                let sites: Vec<&SiteBase> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_owner_id() == player_id)
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
                    .filter_map(|c| c.get_site_base())
                    .collect();
                for site in sites {
                    state.resources.get_mut(player_id).unwrap().mana += site.provided_mana;
                }
            }
            Effect::RemoveResources {
                player_id,
                mana,
                thresholds,
                health,
            } => {
                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana -= mana;
                player_resources.thresholds.air -= thresholds.air;
                player_resources.thresholds.water -= thresholds.water;
                player_resources.thresholds.fire -= thresholds.fire;
                player_resources.thresholds.earth -= thresholds.earth;
                player_resources.health -= health;
            }
            Effect::EndTurn { player_id } => {
                let resources = state.resources.get_mut(player_id).unwrap();
                resources.mana = 0;

                let snapshot = state.snapshot();
                let effects: Vec<Effect> = state.cards.iter_mut().flat_map(|c| c.on_turn_end(&snapshot)).collect();
                state.effects.extend(effects);

                // Clear any counters that expire at the end of the turn
                let card_ids: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_unit())
                    .filter(|c| {
                        !c.get_unit_base()
                            .unwrap_or(&UnitBase::default())
                            .power_counters
                            .is_empty()
                    })
                    .map(|c| c.get_id().clone())
                    .collect();
                for card_id in card_ids {
                    let card = state.get_card_mut(&card_id).unwrap();
                    let base = card.get_unit_base_mut().unwrap();
                    for counter in &mut base.power_counters {
                        if counter.expires_in_turns.is_some() && counter.expires_in_turns.unwrap() > 0 {
                            counter.expires_in_turns = Some(counter.expires_in_turns.unwrap() - 1);
                        }
                    }

                    card.get_unit_base_mut()
                        .unwrap()
                        .power_counters
                        .retain(|c| c.expires_in_turns.is_none() || c.expires_in_turns.unwrap() > 0);
                }
            }
            Effect::AddResources {
                player_id,
                mana,
                thresholds,
                health,
            } => {
                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana += mana;
                player_resources.thresholds.air += thresholds.air;
                player_resources.thresholds.water += thresholds.water;
                player_resources.thresholds.fire += thresholds.fire;
                player_resources.thresholds.earth += thresholds.earth;
                player_resources.health += health;
            }
            Effect::Attack {
                attacker_id,
                defender_id,
            } => {
                let snapshot = state.snapshot();
                let attacker = state.cards.iter().find(|c| c.get_id() == attacker_id).unwrap();
                let defender = state.cards.iter().find(|c| c.get_id() == defender_id).unwrap();
                let effects = vec![
                    Effect::MoveCard {
                        card_id: attacker_id.clone(),
                        from: attacker.get_zone(),
                        to: defender.get_zone(),
                        tap: true,
                    },
                    Effect::TakeDamage {
                        card_id: defender_id.clone(),
                        from: attacker_id.clone(),
                        damage: attacker.get_power(&snapshot).unwrap(),
                    },
                ];
                state.effects.extend(effects);

                // Sites do not strike back
                if let Some(defender_power) = defender.get_power(&snapshot) {
                    state.effects.push_back(Effect::TakeDamage {
                        card_id: attacker_id.clone(),
                        from: defender_id.clone(),
                        damage: defender_power,
                    });
                }

                state.effects.push_back(Effect::wait_for_play(attacker.get_owner_id()));
            }
            Effect::TakeDamage { card_id, damage, from } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let effects = card.on_take_damage(&snapshot, from, *damage);
                state.effects.extend(effects);
            }
            Effect::BuryCard { card_id } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let effects = card.deathrite(&snapshot);
                card.set_zone(Zone::Cemetery);
                state.effects.extend(effects);
            }
            Effect::AddCounter { card_id, counter } => {
                let card = state.get_card_mut(card_id).unwrap();
                if card.is_unit() {
                    let base = card.get_unit_base_mut().unwrap();
                    base.power_counters.push(counter.clone());
                }
            }
            Effect::AddModifier { card_id, counter } => {
                let card = state.get_card_mut(card_id).unwrap();
                if card.is_unit() {
                    let base = card.get_unit_base_mut().unwrap();
                    base.modifier_counters.push(counter.clone());
                }
            }
        }

        Ok(())
    }
}
