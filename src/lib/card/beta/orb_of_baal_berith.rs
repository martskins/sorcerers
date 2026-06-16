use crate::prelude::*;
use std::sync::Arc;

const CREATE_COPY_HOOK: HookId = 1;
const TURN_START_HOOK: HookId = 2;

#[derive(Debug, Clone)]
pub struct OrbOfBaalBerith {
    artifact_base: ArtifactBase,
    card_base: CardBase,
    already_copied_magic_this_turn: bool,
}

impl OrbOfBaalBerith {
    pub const NAME: &'static str = "Orb of Ba’al Berith";
    pub const DESCRIPTION: &'static str = "The first time each turn a Magic spell is cast nearby, Orb of Ba'al Berith creates a copy. The spell's controller may choose new targets.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Monument],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(5),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            already_copied_magic_this_turn: false,
        }
    }
}

impl Artifact for OrbOfBaalBerith {}

#[async_trait::async_trait]
impl Card for OrbOfBaalBerith {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(already_copied_magic_this_turn) = data.downcast_ref::<bool>() {
            self.already_copied_magic_this_turn = *already_copied_magic_this_turn;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Invalid data type for Orb of Ba'al Berith: expected bool"
            ))
        }
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![
            Hook {
                id: CREATE_COPY_HOOK,
                trigger: EffectQuery::PlayCard {
                    card: CardQuery::new().magics(),
                    spellcaster: Some(CardQuery::new().units().nearby_to_card(self.get_id())),
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
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            CREATE_COPY_HOOK => {
                let Effect::PlayMagic {
                    player_id,
                    card_id,
                    caster_id,
                    ..
                } = effect
                else {
                    return Ok(vec![]);
                };

                if self.already_copied_magic_this_turn {
                    return Ok(vec![]);
                }

                Ok(vec![
                    Effect::CopyMagic {
                        source_id: *self.get_id(),
                        player_id: *player_id,
                        card_id: *card_id,
                        caster_id: *caster_id,
                    },
                    Effect::SetCardData {
                        card_id: *self.get_id(),
                        data: Arc::new(true),
                    },
                ])
            }
            TURN_START_HOOK => Ok(vec![Effect::SetCardData {
                card_id: *self.get_id(),
                data: Arc::new(false),
            }]),
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (OrbOfBaalBerith::NAME, |owner_id: PlayerId| {
        Box::new(OrbOfBaalBerith::new(owner_id))
    });
