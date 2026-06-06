use crate::prelude::*;

const UNTAP_AFTER_KILL_HOOK: HookId = 1;
const TURN_START_HOOK: HookId = 2;

#[derive(Debug, Clone)]
pub struct TvinnaxBerserker {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl TvinnaxBerserker {
    pub const NAME: &'static str = "Tvinnax Berserker";
    pub const DESCRIPTION: &'static str = "Whenever Tvinnax Berserker can attack a unit, he must. Untap Tvinnax Berserker whenever he attacks and kills an enemy minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for TvinnaxBerserker {
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
    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![
            Hook {
                id: UNTAP_AFTER_KILL_HOOK,
                trigger: EffectQuery::UnitKilled {
                    unit: CardQuery::new()
                        .minions()
                        .controlled_by_different_controller_than_card(self.get_id()),
                    killer: Some(self.get_id().into()),
                    from_attack: None,
                },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
            Hook {
                id: TURN_START_HOOK,
                trigger: EffectQuery::TurnStart { player_id: None },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
        ])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            UNTAP_AFTER_KILL_HOOK => {
                if let Effect::KillMinion {
                    card_id,
                    killer_id,
                    from_attack: true,
                    ..
                } = effect
                    && killer_id == self.get_id()
                    && state.get_card(card_id).get_controller_id(state)
                        != self.get_controller_id(state)
                {
                    return Ok(vec![Effect::SetTapped {
                        card_id: *self.get_id(),
                        tapped: false,
                    }]);
                }

                Ok(vec![])
            }
            TURN_START_HOOK => {
                if !self.can_attack(state) {
                    return Ok(vec![]);
                }

                let valid_targets = self.get_valid_attack_targets(state, false);
                if valid_targets.is_empty() {
                    return Ok(vec![]);
                }

                let player_id = self.get_controller_id(state);
                let picked_card_id = pick_card(
                    player_id,
                    &valid_targets,
                    state,
                    "Tvinnax Berserker: Choose a unit to attack",
                )
                .await?;
                Ok(vec![Effect::DeclareAttack {
                    attacker_id: *self.get_id(),
                    target_id: picked_card_id,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TvinnaxBerserker::NAME, |owner_id: PlayerId| {
        Box::new(TvinnaxBerserker::new(owner_id))
    });
