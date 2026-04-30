use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

/// **Primordial Spring** — Elite Site (no threshold)
///
/// Genesis → If you control fewer sites than any opponent, draw three sites.
#[derive(Debug, Clone)]
pub struct PrimordialSpring {
    site_base: SiteBase,
    card_base: CardBase,
}

impl PrimordialSpring {
    pub const NAME: &'static str = "Primordial Spring";
    pub const DESCRIPTION: &'static str =
        "Genesis → If you control fewer sites than any opponent, draw three sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for PrimordialSpring {}

#[async_trait::async_trait]
impl Card for PrimordialSpring {
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

        let my_site_count = CardQuery::new()
            .sites()
            .in_play()
            .controlled_by(&controller_id)
            .all(state)
            .len();

        let opponent_has_more = state
            .players
            .iter()
            .filter(|p| p.id != controller_id)
            .any(|p| {
                CardQuery::new()
                    .sites()
                    .in_play()
                    .controlled_by(&p.id)
                    .all(state)
                    .len()
                    > my_site_count
            });

        if opponent_has_more {
            Ok(vec![Effect::DrawSite {
                player_id: controller_id,
                count: 3,
            }])
        } else {
            Ok(vec![])
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PrimordialSpring::NAME, |owner_id: PlayerId| {
        Box::new(PrimordialSpring::new(owner_id))
    });
