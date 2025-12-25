use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card_with_preview},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Observatory {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Observatory {
    pub const NAME: &'static str = "Observatory";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
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
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Observatory {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    async fn genesis(&self, state: &State) -> Vec<Effect> {
        let deck = state.decks.get(self.get_owner_id()).unwrap().clone();
        let mut spells = deck.spells.clone();
        let mut cards = vec![];
        for _ in 0..3 {
            if let Some(card_id) = spells.pop() {
                cards.push(card_id);
            }
        }

        while cards.len() > 0 {
            let position = match cards.len() {
                3 => "third from the top",
                2 => "second from the top",
                1 => "on the top",
                _ => unreachable!(),
            };
            let picked_card_id = pick_card_with_preview(
                self.get_owner_id(),
                &cards,
                state,
                &format!("Pick a spell to put back into your spellbook, {}", position),
            )
            .await;
            spells.push(picked_card_id.clone());

            let idx = cards.iter().position(|id| id == &picked_card_id).unwrap();
            cards.remove(idx);
        }

        vec![Effect::RearrangeDeck {
            spells: spells,
            sites: deck.sites.clone(),
        }]
    }
}
