use crate::{
    card::{Ability, CardType, SiteBase, Zone},
    game::{PlayerId, PlayerStatus, Thresholds},
    state::State,
};
use std::fmt::Debug;

pub trait CardStatus: Debug + Send + Sync {}

#[derive(Debug, Clone)]
pub enum Effect {
    SetPlayerStatus {
        status: PlayerStatus,
    },
    MoveCard {
        card_id: uuid::Uuid,
        to: Zone,
    },
    DrawCard {
        player_id: uuid::Uuid,
        card_type: CardType,
    },
    PlayCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        square: u8,
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
    },
    AddResources {
        player_id: uuid::Uuid,
        mana: u8,
        thresholds: Thresholds,
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
    KillUnit {
        card_id: uuid::Uuid,
    },
}

impl Effect {
    pub fn tap_card(card_id: &uuid::Uuid) -> Self {
        Effect::TapCard {
            card_id: card_id.clone(),
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
            Effect::MoveCard { card_id, to } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(to.clone());
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
            Effect::PlayCard { card_id, square, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(Zone::Realm(*square));
                let mut effects = card.genesis(&snapshot);
                let mana_cost = card.get_mana_cost(&snapshot);
                effects.push(Effect::RemoveResources {
                    player_id: card.get_owner_id().clone(),
                    mana: mana_cost,
                    thresholds: Thresholds::new(),
                });
                state.effects.extend(effects);
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
            } => {
                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana -= mana;
                player_resources.thresholds.air -= thresholds.air;
                player_resources.thresholds.water -= thresholds.water;
                player_resources.thresholds.fire -= thresholds.fire;
                player_resources.thresholds.earth -= thresholds.earth;
            }
            Effect::EndTurn { player_id } => {
                let resources = state.resources.get_mut(player_id).unwrap();
                resources.mana = 0;
            }
            Effect::AddResources {
                player_id,
                mana,
                thresholds,
            } => {
                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana += mana;
                player_resources.thresholds.air += thresholds.air;
                player_resources.thresholds.water += thresholds.water;
                player_resources.thresholds.fire += thresholds.fire;
                player_resources.thresholds.earth += thresholds.earth;
            }
            Effect::Attack {
                attacker_id,
                defender_id,
            } => {
                let snapshot = state.snapshot();
                let attacker = state.cards.iter().find(|c| c.get_id() == attacker_id).unwrap();
                let defender = state.cards.iter().find(|c| c.get_id() == defender_id).unwrap();

                let attacker_power = attacker.get_power(&snapshot).unwrap();
                state.effects.push_back(Effect::TakeDamage {
                    card_id: defender_id.clone(),
                    from: attacker_id.clone(),
                    damage: attacker_power,
                });

                // Sites do not strike back
                if let Some(defender_power) = defender.get_power(&snapshot) {
                    state.effects.push_back(Effect::TakeDamage {
                        card_id: attacker_id.clone(),
                        from: defender_id.clone(),
                        damage: defender_power,
                    });
                }
            }
            Effect::TakeDamage { card_id, damage, from } => {
                let attacker = state.cards.iter().find(|c| c.get_id() == from).unwrap();
                let is_lethal = attacker.get_unit_base().unwrap().abilities.contains(&Ability::Lethal);
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let player_id = card.get_owner_id().clone();
                match card.get_card_type() {
                    CardType::Site => {
                        let resources = state.resources.get_mut(&player_id).unwrap();
                        resources.health -= damage;
                    }
                    CardType::Spell => {
                        if is_lethal {
                            state.effects.push_back(Effect::KillUnit {
                                card_id: card_id.clone(),
                            });
                        } else {
                            card.get_unit_base_mut().unwrap().damage += damage;
                        }
                    }
                    CardType::Avatar => {
                        let resources = state.resources.get_mut(&player_id).unwrap();
                        resources.health -= damage;
                    }
                }
            }
            Effect::KillUnit { card_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(Zone::Cemetery);
            }
        }

        Ok(())
    }
}
