use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct RedDesert {
    site_base: SiteBase,
    card_base: CardBase,
}

impl RedDesert {
    pub const NAME: &'static str = "Red Desert";
    pub const DESCRIPTION: &'static str =
        "Genesis → Deal 1 damage to each minion atop target nearby site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![SiteType::Desert],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for RedDesert {}

impl ResourceProvider for RedDesert {}

#[async_trait::async_trait]
impl Card for RedDesert {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let controller_id = self.get_controller_id(state);
                let Some(picked_site_id) = CardQuery::new()
                    .sites()
                    .near_to(self.get_location())
                    .with_prompt("Pick a site to deal 1 damage to all atop units")
                    .with_source_card(*self.get_id())
                    .pick(&controller_id, state)
                    .await?
                else {
                    return Ok(vec![]);
                };
                let site = state.get_card(&picked_site_id);
                let minions = CardQuery::new()
                    .minions()
                    .in_location(site.get_location().with_region(Region::Surface))
                    .all(state);
                let mut effects = vec![];
                for minion in minions {
                    effects.push(Effect::TakeDamage {
                        card_id: minion,
                        from: *site.get_id(),
                        damage: Damage::basic(1),
                    });
                }
                Ok(effects)
            }
            _ => Ok(vec![]),
        }
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RedDesert::NAME, |owner_id: PlayerId| {
    Box::new(RedDesert::new(owner_id))
});
