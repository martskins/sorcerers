use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        SiteType, Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card_with_preview},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Observatory {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Observatory {
    pub const NAME: &'static str = "Observatory";
    pub const DESCRIPTION: &'static str =
        "Genesis → Look at your next three spells. Put them back in any order.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
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

impl Site for Observatory {}

#[async_trait::async_trait]
impl Card for Observatory {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let deck = state
            .decks
            .get(&self.get_controller_id(state))
            .unwrap()
            .clone();
        let mut spells = deck.spells.clone();
        let mut cards = vec![];
        for _ in 0..3 {
            if let Some(card_id) = spells.pop() {
                cards.push(card_id);
            }
        }

        while !cards.is_empty() {
            let position = match cards.len() {
                3 => "third from the top",
                2 => "second from the top",
                1 => "on the top",
                _ => unreachable!(),
            };
            let picked_card_id = pick_card_with_preview(
                self.get_controller_id(state),
                &cards,
                state,
                &format!("Pick a spell to put back into your spellbook, {}", position),
            )
            .await?;
            spells.push(picked_card_id);

            let idx = cards.iter().position(|id| id == &picked_card_id).unwrap();
            cards.remove(idx);
        }

        Ok(vec![Effect::RearrangeDeck {
            spells,
            sites: deck.sites.clone(),
        }])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Observatory::NAME, |owner_id: PlayerId| {
    Box::new(Observatory::new(owner_id))
});
