use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MagneticMuzzle {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl MagneticMuzzle {
    pub const NAME: &'static str = "Magnetic Muzzle";
    pub const DESCRIPTION: &'static str = "Bearer is silenced and can't drop Magnetic Muzzle. At the end of each player's turn, if Magnetic Muzzle is abandoned, that player attaches it to a nearby minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(2),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for MagneticMuzzle {}

#[async_trait::async_trait]
impl Card for MagneticMuzzle {
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
    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }
    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }
    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }

    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![OngoingEffect::GrantStatus {
            status: CardStatus::Silenced,
            affected_cards: CardQuery::new().bearer_of_card(self.get_id()),
        }])
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
                if !self.get_zone().is_in_play() || self.get_bearer_id()?.is_some() {
                    return Ok(vec![]);
                }

                let player_id = state.current_player();
                let Some(target_id) = CardQuery::new()
                    .minions()
                    .near_to(self.get_location())
                    .with_prompt("Pick a nearby minion to attach")
                    .with_source_card(*self.get_id())
                    .pick(&player_id, state)
                    .await?
                else {
                    return Ok(vec![]);
                };
                let target = state.get_card(&target_id);
                Ok(vec![
                    Effect::MoveCard {
                        player_id,
                        card_id: *self.get_id(),
                        from: self.get_location().clone(),
                        to: LocationQuery::from_location(
                            target
                                .get_location()
                                .with_region(target.get_region(state).clone()),
                        ),
                        tap: false,
                        through_path: None,
                    },
                    Effect::SetBearer {
                        card_id: *self.get_id(),
                        bearer_id: Some(target_id),
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MagneticMuzzle::NAME, |owner_id: PlayerId| {
        Box::new(MagneticMuzzle::new(owner_id))
    });
