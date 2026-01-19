use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, Rarity, Region, Site, SiteBase, SiteType, Zone},
    effect::{AbilityCounter, Effect},
    game::{PlayerId, Thresholds},
    query::{CardQuery, EffectQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
pub struct PlanarGate {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl PlanarGate {
    pub const NAME: &'static str = "Planar Gate";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse(""),
                types: vec![SiteType::Tower],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for PlanarGate {
    fn on_card_enter(&self, state: &State, card_id: &uuid::Uuid) -> Vec<Effect> {
        let card = state.get_card(card_id);
        if !card.is_minion() {
            return vec![];
        }

        vec![Effect::AddAbilityCounter {
            card_id: card_id.clone(),
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Voidwalk,
                expires_on_effect: Some(EffectQuery::EnterZone {
                    card: CardQuery::Specific {
                        id: uuid::Uuid::new_v4(),
                        card_id: card_id.clone(),
                    },
                    zone: ZoneQuery::AnySite {
                        id: uuid::Uuid::new_v4(),
                        controlled_by: None,
                        prompt: None,
                    },
                }),
            },
        }]
    }
}

#[async_trait::async_trait]
impl Card for PlanarGate {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (PlanarGate::NAME, |owner_id: PlayerId| {
    Box::new(PlanarGate::new(owner_id))
});
