use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BlackObelisk {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl BlackObelisk {
    pub const NAME: &'static str = "Black Obelisk";
    pub const DESCRIPTION: &'static str =
        "Black Obelisk's site has “At the start of your turn, lose 2 life and gain ② this turn.”";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Monument],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for BlackObelisk {}

const TURN_START_HOOK: HookId = 1;

#[async_trait::async_trait]
impl Card for BlackObelisk {
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
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![OngoingEffect::ModifyProvidedMana {
            mana_diff: 2,
            affected_cards: CardQuery::new().in_zone_of_card(self.get_id()).sites(),
        }])
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TURN_START_HOOK,
            trigger: EffectQuery::TurnStart { player_id: None },
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
            TURN_START_HOOK => {
                let Some(site): Option<&dyn Site> = self.get_zone().get_site(state) else {
                    return Ok(vec![]);
                };

                let controller_id = site.get_controller_id(state);
                if controller_id != state.current_player() {
                    return Ok(vec![]);
                }

                if site.has_status(state, &CardStatus::Disabled) {
                    return Ok(vec![]);
                }

                Ok(vec![Effect::AdjustAvatarLife {
                    player_id: controller_id,
                    amount: -2,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BlackObelisk::NAME, |owner_id: PlayerId| {
    Box::new(BlackObelisk::new(owner_id))
});
