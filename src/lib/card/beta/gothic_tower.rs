use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct GothicTower {
    site_base: SiteBase,
    card_base: CardBase,
}

impl GothicTower {
    pub const NAME: &'static str = "Gothic Tower";
    pub const DESCRIPTION: &'static str =
        "Genesis → If this is the only Gothic Tower you control, gain ① this turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
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
impl Site for GothicTower {}

impl ResourceProvider for GothicTower {}

#[async_trait::async_trait]
impl Card for GothicTower {
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
                let count = state
                    .cards
                    .values()
                    .filter(|c| c.get_zone().is_in_play())
                    .filter(|c| c.get_id() != self.get_id())
                    .filter(|c| &c.get_controller_id(state) == self.get_owner_id())
                    .filter(|c| c.get_name() == Self::NAME)
                    .count();
                if count > 0 {
                    return Ok(vec![]);
                }

                Ok(vec![Effect::AdjustMana {
                    player_id: *self.get_owner_id(),
                    mana: 1,
                }])
            }
            _ => Ok(vec![]),
        }
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GothicTower::NAME, |owner_id: PlayerId| {
    Box::new(GothicTower::new(owner_id))
});
