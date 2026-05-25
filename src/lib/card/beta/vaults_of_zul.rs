use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct VaultsOfZul {
    site_base: SiteBase,
    card_base: CardBase,
    triggered: bool,
}

impl VaultsOfZul {
    pub const NAME: &'static str = "Vaults of Zul";
    pub const DESCRIPTION: &'static str =
        "The first time an Avatar stops here, they draw three cards and skip their next turn.";

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
            triggered: false,
        }
    }
}

#[async_trait::async_trait]
impl Site for VaultsOfZul {
    fn on_card_stop(&self, state: &State, card_id: &uuid::Uuid) -> Vec<Effect> {
        if self.triggered || !state.get_card(card_id).is_avatar() {
            return vec![];
        }

        let controller_id = state.get_card(card_id).get_controller_id(state);
        vec![
            Effect::SetCardData {
                card_id: *self.get_id(),
                data: std::sync::Arc::new(true),
            },
            Effect::DrawCard {
                player_id: controller_id,
                count: 3,
                kind: DrawKind::Choice,
            },
            Effect::SkipNextTurn {
                player_id: controller_id,
            },
        ]
    }
}

impl ResourceProvider for VaultsOfZul {}

#[async_trait::async_trait]
impl Card for VaultsOfZul {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(triggered) = data.downcast_ref::<bool>() {
            self.triggered = *triggered;
        }

        Ok(())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (VaultsOfZul::NAME, |owner_id: PlayerId| {
    Box::new(VaultsOfZul::new(owner_id))
});
