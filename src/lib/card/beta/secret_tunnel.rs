use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SecretTunnel {
    site_base: SiteBase,
    card_base: CardBase,
}

impl SecretTunnel {
    pub const NAME: &'static str = "Secret Tunnel";
    pub const DESCRIPTION: &'static str =
        "Burrowed allies can move as if this were adjacent to your other sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
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
impl Site for SecretTunnel {}

impl ResourceProvider for SecretTunnel {}

#[async_trait::async_trait]
impl Card for SecretTunnel {
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

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let controller_id = self.get_controller_id(state);
        let connected_locations = CardQuery::new()
            .sites()
            .controlled_by(&controller_id)
            .id_not(*self.get_id())
            .in_play()
            .all(state)
            .into_iter()
            .map(|site_id| state.get_card(&site_id).get_location().clone())
            .collect();

        Ok(vec![OngoingEffect::ConnectZones {
            connected_locations,
            affected_cards: CardQuery::new()
                .units()
                .in_zone_of_card(self.get_id())
                .controlled_by(&self.get_controller_id(state))
                .in_region(Region::Underground),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SecretTunnel::NAME, |owner_id: PlayerId| {
    Box::new(SecretTunnel::new(owner_id))
});
