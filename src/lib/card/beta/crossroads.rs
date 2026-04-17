use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card_with_preview},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Crossroads {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Crossroads {
    pub const NAME: &'static str = "Crossroads";
    pub const DESCRIPTION: &'static str =
        "Genesis → Look at your next four sites. Put three on the bottom of your atlas.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
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

impl Site for Crossroads {}

#[async_trait::async_trait]
impl Card for Crossroads {
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
        let deck = state.decks.get(&controller_id).unwrap().clone();

        let mut remaining_sites = deck.sites.clone();
        let mut looked_at: Vec<uuid::Uuid> = vec![];
        for _ in 0..4 {
            if let Some(card_id) = remaining_sites.pop() {
                looked_at.push(card_id);
            }
        }

        if looked_at.is_empty() {
            return Ok(vec![]);
        }

        let chosen_id = pick_card_with_preview(
            &controller_id,
            &looked_at,
            state,
            "Crossroads: Pick a site to keep on top of your atlas",
        )
        .await?;

        // The rest go to the bottom of the deck (front of sites vec).
        let bottom: Vec<uuid::Uuid> = looked_at
            .iter()
            .filter(|id| **id != chosen_id)
            .cloned()
            .collect();

        // New order: bottom cards at front, then remaining_sites, then chosen on top (end).
        let mut new_sites = bottom;
        new_sites.extend(remaining_sites);
        new_sites.push(chosen_id);

        Ok(vec![Effect::RearrangeDeck {
            spells: deck.spells.clone(),
            sites: new_sites,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Crossroads::NAME, |owner_id: PlayerId| {
    Box::new(Crossroads::new(owner_id))
});
