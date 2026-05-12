use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct RootsOfYggdrasil {
    site_base: SiteBase,
    card_base: CardBase,
}

impl RootsOfYggdrasil {
    pub const NAME: &'static str = "Roots of Yggdrasil";
    pub const DESCRIPTION: &'static str =
        "When the Roots of Yggdrasil are destroyed, destroy everything.";

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
impl Site for RootsOfYggdrasil {}

impl ResourceProvider for RootsOfYggdrasil {}

#[async_trait::async_trait]
impl Card for RootsOfYggdrasil {
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

    fn deathrite(&self, state: &State, _from: &Zone) -> Vec<Effect> {
        CardQuery::new()
            .in_play()
            .card_types(vec![
                CardType::Minion,
                CardType::Artifact,
                CardType::Site,
                CardType::Aura,
            ])
            .all(state)
            .into_iter()
            .map(|card_id| Effect::BuryCard { card_id })
            .collect()
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (RootsOfYggdrasil::NAME, |owner_id: PlayerId| {
        Box::new(RootsOfYggdrasil::new(owner_id))
    });
