use crate::{
    card::{Card, CardBase, Edition, Modifier, Plane, Rarity, Site, SiteBase, SiteType, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct UpdraftRidge {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl UpdraftRidge {
    pub const NAME: &'static str = "Updraft Ridge";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
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

impl Site for UpdraftRidge {}

#[async_trait::async_trait]
impl Card for UpdraftRidge {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn area_modifiers(&self, state: &State) -> Vec<(Modifier, Vec<uuid::Uuid>)> {
        let units: Vec<uuid::Uuid> = self
            .get_zone()
            .get_units(state, None)
            .iter()
            .filter(|c| c.has_modifier(state, &Modifier::Airborne))
            .map(|c| c.get_id().clone())
            .collect();
        vec![(Modifier::Movement(1), units)]
    }
}
