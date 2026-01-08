use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HolyGround {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl HolyGround {
    pub const NAME: &'static str = "Holy Ground";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                types: vec![],
                ..Default::default()
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
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for HolyGround {}

#[async_trait::async_trait]
impl Card for HolyGround {
    fn get_name(&self) -> &str {
        Self::NAME
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let nearby_avatars: Vec<uuid::Uuid> = self
            .get_zone()
            .get_nearby_units(state, None)
            .iter()
            .filter(|c| c.is_avatar())
            .map(|a| a.get_id())
            .cloned()
            .collect();

        let effects = nearby_avatars
            .iter()
            .map(|a| Effect::Heal {
                card_id: a.clone(),
                amount: 3,
            })
            .collect();

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (HolyGround::NAME, |owner_id: PlayerId| {
    Box::new(HolyGround::new(owner_id))
});
