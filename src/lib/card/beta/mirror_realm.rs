use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MirrorRealm {
    site_base: SiteBase,
    card_base: CardBase,
}

impl MirrorRealm {
    pub const NAME: &'static str = "Mirror Realm";
    pub const DESCRIPTION: &'static str =
        "This site enters the realm as a copy of another nearby site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for MirrorRealm {}

impl ResourceProvider for MirrorRealm {}

#[async_trait::async_trait]
impl Card for MirrorRealm {
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

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
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
                let nearby_sites = CardQuery::new()
                    .sites()
                    .near_to(self.get_zone())
                    .id_not(*self.get_id())
                    .all(state);
                if nearby_sites.is_empty() {
                    return Ok(vec![]);
                }

                let picked_site_id = pick_card(
                    self.get_controller_id(state),
                    &nearby_sites,
                    state,
                    "Mirror Realm: Pick a nearby site to copy",
                )
                .await?;

                Ok(vec![Effect::MakeCardCopyOf {
                    card_id: *self.get_id(),
                    copy_source_id: picked_site_id,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MirrorRealm::NAME, |owner_id: PlayerId| {
    Box::new(MirrorRealm::new(owner_id))
});
