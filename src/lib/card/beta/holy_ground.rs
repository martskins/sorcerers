use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct HolyGround {
    site_base: SiteBase,
    card_base: CardBase,
}

impl HolyGround {
    pub const NAME: &'static str = "Holy Ground";
    pub const DESCRIPTION: &'static str = "Genesis → Each nearby Avatar heals 3 life.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
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
impl Site for HolyGround {}

impl ResourceProvider for HolyGround {}

#[async_trait::async_trait]
impl Card for HolyGround {
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
                let effects = CardQuery::new()
                    .near_to(self.get_zone())
                    .card_types(vec![CardType::Avatar])
                    .all(state)
                    .iter()
                    .map(|a| Effect::AdjustAvatarLife {
                        player_id: state.get_card(a).get_controller_id(state),
                        amount: 3,
                    })
                    .collect();

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (HolyGround::NAME, |owner_id: PlayerId| {
    Box::new(HolyGround::new(owner_id))
});
