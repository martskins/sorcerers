use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ShiftingSands {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl ShiftingSands {
    pub const NAME: &'static str = "Shifting Sands";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![SiteType::Desert],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for ShiftingSands {}

#[async_trait::async_trait]
impl Card for ShiftingSands {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    async fn genesis(&self, state: &State) -> Vec<Effect> {
        let mut effects = vec![];
        let nearby_sites: Vec<&Box<dyn Card>> = self
            .get_zone()
            .get_nearby_sites(state, Some(self.get_owner_id()))
            .iter()
            .cloned()
            .filter(|c| c.get_site_base().unwrap().types.contains(&SiteType::Desert))
            .collect();
        for site in nearby_sites {
            effects.extend(site.genesis(state).await);
        }
        effects
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }
}
