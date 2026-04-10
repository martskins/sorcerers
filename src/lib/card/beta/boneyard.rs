use crate::{
    card::{Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, Site, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card_with_options},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct Boneyard {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Boneyard {
    pub const NAME: &'static str = "Boneyard";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
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
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for Boneyard {}

#[async_trait::async_trait]
impl Card for Boneyard {
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
        let mut cards = vec![];
        for player in &state.players {
            let player_id = player.id.clone();
            let state = state.snapshot();
            let zone = self.get_zone().clone();

            let minions = &CardMatcher::new()
                .in_zone(&Zone::Cemetery)
                .with_controller_id(&player_id)
                .resolve_ids(&state);
            let picked_minion_id = pick_card_with_options(
                &player_id,
                &minions,
                &minions,
                true,
                &state,
                "Pick a minion in your cemetery to summon to Boneyard",
            )
            .await?;

            cards.push((player_id.clone(), picked_minion_id, zone.clone()));
        }

        Ok(vec![Effect::SummonCards { cards }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Boneyard::NAME, |owner_id: PlayerId| Box::new(Boneyard::new(owner_id)));
