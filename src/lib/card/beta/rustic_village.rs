use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Site, SiteBase, SiteType, Zone},
    effect::{Effect, TokenType},
    game::{BaseOption, PlayerId, Thresholds, pick_option},
    state::State,
};

#[derive(Debug, Clone)]
pub struct RusticVillage {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl RusticVillage {
    pub const NAME: &'static str = "Rustic Village";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                types: vec![SiteType::Village],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for RusticVillage {}

#[async_trait::async_trait]
impl Card for RusticVillage {
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
        let options = vec![BaseOption::Yes, BaseOption::No];
        let option_labels: Vec<String> = options.iter().map(|o| o.to_string()).collect();
        let picked_option = pick_option(
            self.get_controller_id(state),
            &option_labels,
            state,
            "Humble Village: Pay 1 to summon a foot soldier?",
        )
        .await?;
        if options[picked_option] == BaseOption::No {
            return Ok(vec![]);
        }

        Ok(vec![
            Effect::RemoveResources {
                player_id: self.get_controller_id(state).clone(),
                mana: 1,
            },
            Effect::SummonToken {
                player_id: self.get_controller_id(state).clone(),
                token_type: TokenType::FootSoldier,
                zone: self.get_zone().clone(),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (RusticVillage::NAME, |owner_id: PlayerId| {
    Box::new(RusticVillage::new(owner_id))
});
