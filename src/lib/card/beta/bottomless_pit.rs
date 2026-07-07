use crate::prelude::*;

const KILL_ENTERING_MINION_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct BottomlessPit {
    site_base: SiteBase,
    card_base: CardBase,
}

impl BottomlessPit {
    pub const NAME: &'static str = "Bottomless Pit";
    pub const DESCRIPTION: &'static str =
        "Whenever a non-Airborne minion enters this site, kill it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for BottomlessPit {}

impl ResourceProvider for BottomlessPit {}

#[async_trait::async_trait]
impl Card for BottomlessPit {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: KILL_ENTERING_MINION_HOOK,
            trigger: EffectQuery::EnterLocation {
                card: Box::new(CardQuery::new()
                    .minions()
                    .without_ability(Ability::Airborne)),
                // TODO: Should we differentiate queries from pickers?
                // Maybe we need a LocationQuery and a LocationPicker. The latter wraps a Query and
                // lets the user choose one, the former just acts as a matcher.
                location: Box::new(LocationQuery::from_locations(
                    self.get_location().in_all_regions(),
                )),
                from: None,
            },
            timing: HookTiming::After,
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
            KILL_ENTERING_MINION_HOOK => match effect {
                Effect::SummonCards { summoned_cards } => {
                    let mut effects = vec![];
                    for sc in summoned_cards {
                        if sc.to_location.square() == self.get_location().square() {
                            effects.push(Effect::KillMinion {
                                card_id: sc.card_id,
                                killer_id: *self.get_id(),
                                from_attack: false,
                            })
                        }
                    }

                    Ok(effects)
                }
                Effect::MoveCard { card_id, .. } => Ok(vec![Effect::KillMinion {
                    card_id: *card_id,
                    killer_id: *self.get_id(),
                    from_attack: false,
                }]),
                _ => Ok(vec![]),
            },
            _ => Ok(vec![]),
        }
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BottomlessPit::NAME, |owner_id: PlayerId| {
        Box::new(BottomlessPit::new(owner_id))
    });
