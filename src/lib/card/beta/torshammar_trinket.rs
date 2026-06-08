use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct TorshammarTrinket {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl TorshammarTrinket {
    pub const NAME: &'static str = "Torshammar Trinket";
    pub const DESCRIPTION: &'static str =
        "Bearer has +1 power. After each turn, return this to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(1),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for TorshammarTrinket {}

#[async_trait::async_trait]
impl Card for TorshammarTrinket {
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
        Ok(vec![OngoingEffect::ModifyPower {
            power_diff: 1,
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
        _state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TURN_END_HOOK => {
                let zone = self.get_zone();
                if !zone.is_in_play() {
                    return Ok(vec![]);
                }

                Ok(vec![
                    Effect::SetBearer {
                        card_id: *self.get_id(),
                        bearer_id: None,
                    },
                    Effect::SetCardZone {
                        card_id: *self.get_id(),
                        zone: Zone::Hand,
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TorshammarTrinket::NAME, |owner_id: PlayerId| {
        Box::new(TorshammarTrinket::new(owner_id))
    });
