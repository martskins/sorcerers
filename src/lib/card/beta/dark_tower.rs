use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DarkTower {
    site_base: SiteBase,
    card_base: CardBase,
}

impl DarkTower {
    pub const NAME: &'static str = "Dark Tower";
    pub const DESCRIPTION: &'static str =
        "Genesis → If this is the only Dark Tower you control, gain ① this turn.";

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
impl Site for DarkTower {}

impl ResourceProvider for DarkTower {}

#[async_trait::async_trait]
impl Card for DarkTower {
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
                let dark_towers = CardQuery::new()
                    .sites()
                    .controlled_by(&self.get_controller_id(state))
                    .named(Self::NAME.to_string())
                    .all(state)
                    .len();
                if dark_towers > 1 {
                    return Ok(vec![]);
                }

                Ok(vec![Effect::AdjustMana {
                    player_id: *self.get_owner_id(),
                    amount: 1,
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
static CONSTRUCTOR: (&'static str, CardConstructor) = (DarkTower::NAME, |owner_id: PlayerId| {
    Box::new(DarkTower::new(owner_id))
});
