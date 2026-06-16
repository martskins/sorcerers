use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Mudflow {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Mudflow {
    pub const NAME: &'static str = "Mudflow";
    pub const DESCRIPTION: &'static str =
        "At the start of your turn, surface or unburrow each minion occupying target site nearby.";

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
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for Mudflow {}

impl ResourceProvider for Mudflow {}

const TURN_START_HOOK: HookId = 1;

#[async_trait::async_trait]
impl Card for Mudflow {
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

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
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
                let controller_id = self.get_controller_id(state);
                if state.current_player() != controller_id {
                    return Ok(vec![]);
                }
                if !self.get_zone().is_in_play() {
                    return Ok(vec![]);
                }

                let Some(target_zone_id) = CardQuery::new()
                    .sites()
                    .near_to(self.get_location())
                    .with_prompt("Pick a nearby site to surface/unburrow all minions")
                    .with_source_card(*self.get_id())
                    .pick(&controller_id, state)
                    .await?
                else {
                    return Ok(vec![]);
                };

                let target_site = state.get_card(&target_zone_id);
                let target_zone = target_site.get_zone().clone();

                let minions = CardQuery::new().minions().in_zone(&target_zone).all(state);

                let effects = minions
                    .into_iter()
                    .map(|minion_id| Effect::SetCardRegion {
                        card_id: minion_id,
                        destination: Region::Surface,
                        tap: false,
                    })
                    .collect();

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Mudflow::NAME, |owner_id: PlayerId| {
    Box::new(Mudflow::new(owner_id))
});
