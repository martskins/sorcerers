use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DoomsdayDevice {
    artifact_base: ArtifactBase,
    card_base: CardBase,
    doom_counters: u8,
}

const TURN_END_HOOK: HookId = 1;

impl DoomsdayDevice {
    pub const NAME: &'static str = "Doomsday Device";
    pub const DESCRIPTION: &'static str = "Doomsday Device enters the realm with 6 counters. At the end of each player's turn, remove a counter. When the last is removed, it detonates! Deals damage to each unit at affected locations:";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Device],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(4),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            doom_counters: 0,
        }
    }
}

impl Artifact for DoomsdayDevice {}

#[async_trait::async_trait]
impl Card for DoomsdayDevice {
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
        if let Some(val) = data.downcast_ref::<u8>() {
            self.doom_counters = *val;
        }
        Ok(())
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![
            Hook::genesis(self.get_id()),
            Hook {
                id: TURN_END_HOOK,
                trigger: EffectQuery::TurnEnd { player_id: None },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
        ])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => Ok(vec![Effect::SetCardData {
                card_id: *self.get_id(),
                data: std::sync::Arc::new(6u8),
            }]),
            TURN_END_HOOK => {
                if !self.get_zone().is_in_play() {
                    return Ok(vec![]);
                }

                if self.doom_counters == 0 {
                    return Ok(vec![]);
                }

                if self.doom_counters == 1 {
                    // Trigger the explosion.
                    let explosion_zones: Vec<Zone> = std::iter::once(self.get_location().clone())
                        .chain(self.get_location().get_nearby(state))
                        .map(Zone::from)
                        .collect();

                    let mut effects: Vec<Effect> = CardQuery::new()
                        .units()
                        .in_zones(&explosion_zones)
                        .all(state)
                        .into_iter()
                        .map(|id| Effect::TakeDamage {
                            card_id: id,
                            from: *self.get_id(),
                            damage: Damage::basic(6),
                        })
                        .collect();

                    effects.push(Effect::BuryCard {
                        card_id: *self.get_id(),
                    });
                    return Ok(effects);
                }

                Ok(vec![Effect::SetCardData {
                    card_id: *self.get_id(),
                    data: std::sync::Arc::new(self.doom_counters - 1),
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (DoomsdayDevice::NAME, |owner_id: PlayerId| {
        Box::new(DoomsdayDevice::new(owner_id))
    });
