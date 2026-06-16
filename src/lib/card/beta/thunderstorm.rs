use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Thunderstorm {
    aura_base: AuraBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl Thunderstorm {
    pub const NAME: &'static str = "Thunderstorm";
    pub const DESCRIPTION: &'static str = "At the end of your turn, deal 3 damage to a random unit atop affected sites, then you may move Thunderstorm one step.\r \r Lasts 3 of your turns.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
        }
    }
}

impl Aura for Thunderstorm {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        let controller_id = self.get_controller_id(state);
        let turns_in_play = state
            .effect_log()
            .iter()
            .skip_while(|e| !matches!(e.effect, Effect::PlayCard { ref card_id, .. } if card_id == self.get_id()))
            .filter(|e| matches!(e.effect, Effect::EndTurn { ref player_id, .. } if player_id == &controller_id))
            .count();

        Ok(turns_in_play >= 3)
    }
}

#[async_trait::async_trait]
impl Card for Thunderstorm {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TURN_END_HOOK,
            trigger: EffectQuery::TurnEnd { player_id: None },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TURN_END_HOOK => {
                if state.current_player() != self.get_controller_id(state) {
                    return Ok(vec![]);
                }

                let zones = self.get_valid_move_locations(state).await?;
                let affected_locations = self.get_affected_zones(state);
                let picked_card_id = CardQuery::new()
                    .randomised()
                    .count(1)
                    .units()
                    .in_locations(&affected_locations)
                    .id_not_in(vec![*self.get_id()])
                    .pick(&self.get_controller_id(state), state)
                    .await?;
                let mut effects = vec![Effect::MoveCard {
                    player_id: self.get_controller_id(state),
                    card_id: *self.get_id(),
                    from: self.get_location().clone(),
                    to: LocationQuery::from_locations(zones)
                        .with_prompt("Pick a zone to move Thunderstorm to"),
                    tap: false,
                    through_path: None,
                }];

                if let Some(picked_card_id) = picked_card_id {
                    effects.push(Effect::TakeDamage {
                        card_id: picked_card_id,
                        from: *self.get_id(),
                        damage: Damage::basic(3),
                    });
                };

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Thunderstorm::NAME, |owner_id: PlayerId| {
    Box::new(Thunderstorm::new(owner_id))
});
