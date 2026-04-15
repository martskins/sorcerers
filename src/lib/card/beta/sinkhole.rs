use crate::{
    card::{Card, CardBase, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct DestroyNearbySite;

#[async_trait::async_trait]
impl ActivatedAbility for DestroyNearbySite {
    fn get_name(&self) -> String {
        "Destroy Nearby Site".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let controller_id = card.get_controller_id(state);
        let Some(picked_card_id) = CardQuery::new()
            .sites()
            .near_to(card.get_zone())
            .with_prompt("Sinkhole: Pick a site to destroy")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        Ok(vec![
            Effect::BuryCard {
                card_id: card_id.clone(),
            },
            Effect::BuryCard {
                card_id: picked_card_id,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct Sinkhole {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Sinkhole {
    pub const NAME: &'static str = "Sinkhole";
    pub const DESCRIPTION: &'static str = "Sacrifice Sinkhole → Destroy a nearby site.";

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
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for Sinkhole {}

#[async_trait::async_trait]
impl Card for Sinkhole {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(DestroyNearbySite)])
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Sinkhole::NAME, |owner_id: PlayerId| {
        Box::new(Sinkhole::new(owner_id))
    });
