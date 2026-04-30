use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, ResourceProvider, Site,
        SiteBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Mirage {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Mirage {
    pub const NAME: &'static str = "Mirage";
    pub const DESCRIPTION: &'static str =
        "Genesis → Swap Mirage with one of your other sites in play.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![],
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

impl Site for Mirage {}

#[async_trait::async_trait]
impl Card for Mirage {
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
        let mirage_zone = self.get_zone().clone();

        let Some(other_site_id) = CardQuery::new()
            .sites()
            .id_not(self.get_id())
            .controlled_by(&controller_id)
            .with_prompt("Mirage: Pick another site to swap with")
            .pick(&controller_id, state, true)
            .await?
        else {
            return Ok(vec![]);
        };

        let other_site = state.get_card(&other_site_id);
        let other_zone = other_site.get_zone().clone();

        Ok(vec![
            // Move the picked site back to atlasbook
            Effect::MoveCard {
                player_id: controller_id,
                card_id: other_site_id,
                from: other_zone.clone(),
                to: ZoneQuery::from_zone(Zone::Atlasbook),
                tap: false,
                region: Region::Surface,
                through_path: None,
            },
            // Move Mirage to the zone the picked site occupied
            Effect::MoveCard {
                player_id: controller_id,
                card_id: *self.get_id(),
                from: mirage_zone,
                to: ZoneQuery::from_zone(other_zone),
                tap: false,
                region: Region::Surface,
                through_path: None,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Mirage::NAME, |owner_id: PlayerId| {
    Box::new(Mirage::new(owner_id))
});
