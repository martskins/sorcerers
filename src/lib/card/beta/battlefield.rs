use crate::{
    card::{Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, Site, SiteBase, Zone},
    effect::{Effect, TokenType},
    game::{PlayerId, Thresholds, pick_option},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Battlefield {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Battlefield {
    pub const NAME: &'static str = "Battlefield";

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
        let choice = pick_option(
            &controller_id,
            &["Broken Weapon".to_string(), "Broken Armor".to_string()],
            state,
            "Battlefield: Choose a broken Weapon or Armor to conjure here",
            false,
        )
        .await?;

        let token_type = if choice == 0 {
            TokenType::BrokenWeapon
        } else {
            TokenType::BrokenArmor
        };

        Ok(vec![Effect::SummonToken {
            player_id: controller_id,
            token_type,
            zone: self.get_zone().clone(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Battlefield::NAME, |owner_id: PlayerId| {
    Box::new(Battlefield::new(owner_id))
});
