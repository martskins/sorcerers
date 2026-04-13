use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct AtlasWanderers {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AtlasWanderers {
    pub const NAME: &'static str = "Atlas Wanderers";
    pub const DESCRIPTION: &'static str = "Genesis → This site and an adjacent site change places, carrying along everything of normal size.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![],
                types: vec![MinionType::Giant],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                costs: Costs::basic(5, "EEE"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for AtlasWanderers {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(picked_site_id) = CardQuery::new()
            .sites()
            .adjacent_to(self.get_zone())
            .with_prompt("Atlas Wanderers: Pick an adjacent site to swap with")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let picked_site = state.get_card(&picked_site_id);
        let from_zone = self.get_zone().clone();
        let to_zone = picked_site.get_zone().clone();

        let mut effects = Vec::new();
        let from_cards = state
            .get_cards_in_zone(&from_zone)
            .iter()
            .filter(|card| !card.is_oversized())
            .map(|card| card.get_id().clone())
            .collect::<Vec<_>>();
        let to_cards = state
            .get_cards_in_zone(&to_zone)
            .iter()
            .filter(|card| !card.is_oversized())
            .map(|card| card.get_id().clone())
            .collect::<Vec<_>>();

        for card_id in from_cards {
            effects.push(Effect::SetCardZone {
                card_id,
                zone: to_zone.clone(),
            });
        }

        for card_id in to_cards {
            effects.push(Effect::SetCardZone {
                card_id,
                zone: from_zone.clone(),
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (AtlasWanderers::NAME, |owner_id: PlayerId| {
        Box::new(AtlasWanderers::new(owner_id))
    });
