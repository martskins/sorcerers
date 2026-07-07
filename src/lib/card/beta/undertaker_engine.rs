use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct UndertakerEngine {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl UndertakerEngine {
    pub const NAME: &'static str = "Undertaker Engine";
    pub const DESCRIPTION: &'static str = "At the end of your turn, you may burrow and/or unburrow any combination of artifacts and minions at this site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                power: Some(4),
                toughness: Some(4),
                types: vec![ArtifactType::Automaton],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(7),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for UndertakerEngine {}

#[async_trait::async_trait]
impl Card for UndertakerEngine {
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
    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }
    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }
    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
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
                let controller_id = self.get_controller_id(state);
                if state.current_player() != controller_id || !self.get_zone().is_in_play() {
                    return Ok(vec![]);
                }

                let cards_here = CardQuery::new()
                    .in_locations(&[
                        self.get_location().with_region(Region::Surface),
                        self.get_location().with_region(Region::Underground),
                    ])
                    .card_types(vec![CardType::Minion, CardType::Artifact])
                    .all(state);
                if cards_here.is_empty() {
                    return Ok(vec![]);
                }

                let selected = pick_cards(
                    &controller_id,
                    &cards_here,
                    state,
                    "Undertaker Engine: Pick any artifacts and minions to burrow or unburrow",
                )
                .await?;

                let mut effects = vec![];
                for card_id in selected {
                    let card = state.get_card(&card_id);
                    let from_region = card.get_region(state).clone();
                    let to_region = if from_region == Region::Underground {
                        Region::Surface
                    } else {
                        Region::Underground
                    };

                    if card.is_minion()
                        && from_region == Region::Surface
                        && !card.has_ability(state, &Ability::Burrowing)
                    {
                        effects.push(Effect::AddAbilityCounter {
                            card_id,
                            counter: AbilityCounter {
                                id: uuid::Uuid::new_v4(),
                                ability: Ability::Burrowing,
                                expires_on_effect: Some(Box::new(EffectQuery::SetCardRegion {
                                    card: card_id.into(),
                                    destination: Some(Region::Surface),
                                })),
                            },
                        });
                    }

                    effects.push(Effect::SetCardRegion {
                        card_id,
                        destination: to_region,
                        tap: false,
                    });
                }

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (UndertakerEngine::NAME, |owner_id: PlayerId| {
        Box::new(UndertakerEngine::new(owner_id))
    });
