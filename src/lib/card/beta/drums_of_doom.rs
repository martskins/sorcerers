use crate::prelude::*;

const DAMAGE_DEALT_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct DrumsOfDoom {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl DrumsOfDoom {
    pub const NAME: &'static str = "Drums of Doom";
    pub const DESCRIPTION: &'static str = "Damage dealt to minions nearby is lethal.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Instrument],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(5),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for DrumsOfDoom {}

#[async_trait::async_trait]
impl Card for DrumsOfDoom {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: DAMAGE_DEALT_HOOK,
            trigger: EffectQuery::DamageDealt {
                source: None,
                target: Some(CardQuery::new().minions().nearby_to_card(self.get_id())),
            },
            timing: HookTiming::Replace,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            DAMAGE_DEALT_HOOK => {
                let Effect::TakeDamage {
                    card_id,
                    from,
                    damage,
                } = effect
                else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::TakeDamage {
                    card_id: *card_id,
                    from: *from,
                    damage: Damage {
                        amount: damage.amount,
                        is_attack: damage.is_attack,
                        is_ranged: damage.is_ranged,
                        is_lethal: true,
                        is_strike: damage.is_strike,
                    },
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (DrumsOfDoom::NAME, |owner_id: PlayerId| {
    Box::new(DrumsOfDoom::new(owner_id))
});
