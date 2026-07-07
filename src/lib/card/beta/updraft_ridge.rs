use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct UpdraftRidge {
    site_base: SiteBase,
    card_base: CardBase,
}

impl UpdraftRidge {
    pub const NAME: &'static str = "Updraft Ridge";
    pub const DESCRIPTION: &'static str = "Airborne minions atop Updraft Ridge move freely away.";

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
impl Site for UpdraftRidge {}

impl ResourceProvider for UpdraftRidge {}

#[async_trait::async_trait]
impl Card for UpdraftRidge {
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

    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![OngoingEffect::GrantAbility {
            ability: Ability::Movement(1),
            affected_cards: Box::new(CardQuery::new()
                .units()
                .in_zone_of_card(self.get_id())
                .with_abilities(vec![Ability::Airborne])),
        }])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (UpdraftRidge::NAME, |owner_id: PlayerId| {
    Box::new(UpdraftRidge::new(owner_id))
});
