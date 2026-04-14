use crate::{
    card::{
        Card, CardBase, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase, Zone,
    },
    effect::{Effect, TokenType},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct TadpolePool {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl TadpolePool {
    pub const NAME: &'static str = "Tadpole Pool";
    pub const DESCRIPTION: &'static str =
        "(W)(W)(W) — Genesis → Summon three submerged Frog tokens here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
                types: vec![],
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
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for TadpolePool {}

#[async_trait::async_trait]
impl Card for TadpolePool {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let resources = state.get_player_resources(&self.get_controller_id(state))?;
        if resources.thresholds.water < 3 {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        Ok(vec![
            Effect::SummonToken {
                player_id: controller_id.clone(),
                token_type: TokenType::Frog,
                zone: self.get_zone().clone(),
            },
            Effect::SummonToken {
                player_id: controller_id.clone(),
                token_type: TokenType::Frog,
                zone: self.get_zone().clone(),
            },
            Effect::SummonToken {
                player_id: controller_id.clone(),
                token_type: TokenType::Frog,
                zone: self.get_zone().clone(),
            },
        ])
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (TadpolePool::NAME, |owner_id: PlayerId| {
        Box::new(TadpolePool::new(owner_id))
    });
