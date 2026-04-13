use crate::{
    card::{ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, Site, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Battlefield {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Battlefield {
    pub const NAME: &'static str = "Battlefield";
    pub const DESCRIPTION: &'static str = "Genesis → Conjure a broken Weapon or Armor here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 0,
                provided_thresholds: Thresholds::ZERO,
                types: vec![],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for Battlefield {}

#[async_trait::async_trait]
impl Card for Battlefield {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(picked_card_id) = CardQuery::new()
            .in_zone(&Zone::Cemetery)
            .artifact_types(vec![ArtifactType::Weapon, ArtifactType::Armor])
            .with_prompt("Battlefield: Pick a weapon or armor to conjure")
            .pick(&controller_id, state, true)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::SummonCard {
            player_id: controller_id,
            card_id: picked_card_id,
            zone: self.get_zone().clone(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Battlefield::NAME, |owner_id: PlayerId| {
    Box::new(Battlefield::new(owner_id))
});
