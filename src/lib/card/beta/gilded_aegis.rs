use crate::prelude::*;

const SAVE_BEARER_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct GildedAegis {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl GildedAegis {
    pub const NAME: &'static str = "Gilded Aegis";
    pub const DESCRIPTION: &'static str =
        "If bearer is a minion and would die, instead fully heal it and banish Gilded Aegis.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Armor],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(2),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for GildedAegis {}

#[async_trait::async_trait]
impl Card for GildedAegis {
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

    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let Some(bearer_id) = self.get_bearer_id()? else {
            return Ok(vec![]);
        };
        if !state.get_card(&bearer_id).is_minion() {
            return Ok(vec![]);
        }

        Ok(vec![Hook {
            id: SAVE_BEARER_HOOK,
            trigger: EffectQuery::UnitKilled {
                unit: bearer_id.into(),
                killer: None,
                from_attack: None,
            },
            timing: HookTiming::Replace,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            SAVE_BEARER_HOOK => {
                let Effect::KillMinion { card_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let bearer = state.get_card(card_id);
                Ok(vec![
                    Effect::BanishCard {
                        card_id: *self.get_id(),
                    },
                    Effect::Heal {
                        card_id: *card_id,
                        amount: bearer.get_toughness(state).unwrap_or_default(),
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GildedAegis::NAME, |owner_id: PlayerId| {
    Box::new(GildedAegis::new(owner_id))
});
