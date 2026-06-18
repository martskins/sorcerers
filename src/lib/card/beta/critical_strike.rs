use crate::prelude::*;

const STRIKE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct CriticalStrike {
    card_base: CardBase,
}

impl CriticalStrike {
    pub const NAME: &'static str = "Critical Strike";
    pub const DESCRIPTION: &'static str =
        "The next time an ally strikes a unit this turn, it deals double damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CriticalStrike {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            STRIKE_HOOK => {
                let Effect::Strike {
                    striker_id,
                    target_id,
                    damage,
                } = effect
                else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::Strike {
                    striker_id: *striker_id,
                    target_id: *target_id,
                    damage: damage.clone() * 2,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl Magic for CriticalStrike {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                hook_id: STRIKE_HOOK,
                card_id: *self.get_id(),
                timing: HookTiming::Replace,
                trigger_on_effect: EffectQuery::StrikeCard {
                    card: CardQuery::new().units(),
                    striker: Some(CardQuery::new().units().controlled_by(&controller_id)),
                },
                expires_on_effect: Some(EffectQuery::TurnEnd {
                    player_id: Some(controller_id),
                }),
                trigger_times: Some(1),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (CriticalStrike::NAME, |owner_id: PlayerId| {
        Box::new(CriticalStrike::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        card::{BoskTroll, FootSoldier, Sorcerer},
        deck::Deck,
        effect::FightContext,
        state::{Player, PlayerWithDeck},
    };

    fn make_state() -> (State, async_channel::Receiver<crate::networking::message::ServerMessage>)
    {
        let player_one_id = uuid::Uuid::new_v4();
        let player_two_id = uuid::Uuid::new_v4();
        let avatar_one = Sorcerer::new(player_one_id);
        let avatar_one_id = *avatar_one.get_id();
        let avatar_two = Sorcerer::new(player_two_id);
        let avatar_two_id = *avatar_two.get_id();

        let player1 = PlayerWithDeck {
            player: Player {
                id: player_one_id,
                name: "Player 1".to_string(),
            },
            deck: Deck::new(
                &player_one_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_one_id,
            ),
            cards: vec![Box::new(avatar_one)],
        };
        let player2 = PlayerWithDeck {
            player: Player {
                id: player_two_id,
                name: "Player 2".to_string(),
            },
            deck: Deck::new(
                &player_two_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_two_id,
            ),
            cards: vec![Box::new(avatar_two)],
        };

        let (server_tx, server_rx) = async_channel::unbounded();
        let (_client_tx, client_rx) = async_channel::unbounded();
        (
            State::new(
                uuid::Uuid::new_v4(),
                vec![player1, player2],
                server_tx,
                client_rx,
            ),
            server_rx,
        )
    }

    #[tokio::test]
    async fn critical_strike_doubles_attack_fight_strike_damage() {
        let (mut state, _server_rx) = make_state();
        let player_id = state.players[0].id;
        let opponent_id = state.players[1].id;

        let critical_strike = CriticalStrike::new(player_id);
        state.add_card(Box::new(critical_strike.clone()));
        let effects = critical_strike
            .resolve_magic(&state, critical_strike.get_id(), Cost::ZERO)
            .await
            .expect("critical strike should resolve");
        state.queue(effects);
        state
            .apply_effects_without_log()
            .await
            .expect("modifier should be added");

        let mut striker = FootSoldier::new(player_id);
        let striker_id = *striker.get_id();
        striker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
        state.add_card(Box::new(striker));

        let mut target = BoskTroll::new(opponent_id);
        let target_id = *target.get_id();
        target.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
        target.add_status(CardStatus::Disabled);
        state.add_card(Box::new(target));

        state.queue_one(Effect::Fight {
            attacker_id: striker_id,
            defender_id: target_id,
            defending_ids: vec![],
            damage_assignment: None,
            context: FightContext::Attack,
        });
        state
            .apply_effects_without_log()
            .await
            .expect("strike should resolve");

        assert_eq!(state.get_card(&target_id).get_damage_taken().unwrap(), 2);
        assert!(state.temporary_effects().is_empty());
    }
}
