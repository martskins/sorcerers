use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, SiteType, Zone},
    effect::{Effect, TokenType},
    game::{BaseOption, PlayerId, Thresholds, pick_option},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HumbleVillage {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl HumbleVillage {
    pub const NAME: &'static str = "Humble Village";

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
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for HumbleVillage {}

#[async_trait::async_trait]
impl Card for HumbleVillage {
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
            self.get_controller_id(),
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
                player_id: self.get_controller_id().clone(),
                mana: 1,
                thresholds: Thresholds::new(),
            },
            Effect::SummonToken {
                player_id: self.get_controller_id().clone(),
                token_type: TokenType::FootSoldier,
                zone: self.get_zone().clone(),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (HumbleVillage::NAME, |owner_id: PlayerId| {
    Box::new(HumbleVillage::new(owner_id))
});
