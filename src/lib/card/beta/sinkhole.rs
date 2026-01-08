use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, Zone},
    effect::Effect,
    game::{CardAction, PlayerId, Thresholds, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
enum SinkholeAbility {
    DestroyNearbySite,
}

#[async_trait::async_trait]
impl CardAction for SinkholeAbility {
    fn get_name(&self) -> &str {
        match self {
            SinkholeAbility::DestroyNearbySite => "Destroy Nearby Site",
        }
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            SinkholeAbility::DestroyNearbySite => {
                let card = state.get_card(card_id);
                let nearby_sites: Vec<uuid::Uuid> = card
                    .get_zone()
                    .get_nearby_sites(state, None)
                    .iter()
                    .map(|c| c.get_id())
                    .cloned()
                    .collect::<Vec<_>>();
                let picked_card_id = pick_card(player_id, &nearby_sites, state, "Select a site to destroy").await?;
                let picked_card = state.get_card(&picked_card_id);
                Ok(vec![
                    Effect::BuryCard {
                        card_id: card_id.clone(),
                        from: card.get_zone().clone(),
                    },
                    Effect::BuryCard {
                        card_id: picked_card_id,
                        from: picked_card.get_zone().clone(),
                    },
                ])
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sinkhole {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Sinkhole {
    pub const NAME: &'static str = "Sinkhole";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
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
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    fn get_actions(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn CardAction>>> {
        Ok(vec![Box::new(SinkholeAbility::DestroyNearbySite)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Sinkhole::NAME, |owner_id: PlayerId| Box::new(Sinkhole::new(owner_id)));
