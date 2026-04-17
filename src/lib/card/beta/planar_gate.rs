use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site,
        SiteBase, SiteType, Zone,
    },
    effect::{AbilityCounter, Effect},
    game::{PlayerId, Thresholds},
    query::{EffectQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
pub struct PlanarGate {
    site_base: SiteBase,
    card_base: CardBase,
}

impl PlanarGate {
    pub const NAME: &'static str = "Planar Gate";
    pub const DESCRIPTION: &'static str =
        "Minions here can traverse the void, gaining Voidwalk until leaving it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse(""),
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

impl Site for PlanarGate {
    fn on_card_enter(&self, state: &State, card_id: &uuid::Uuid) -> Vec<Effect> {
        let card = state.get_card(card_id);
        if !card.is_minion() {
            return vec![];
        }

        vec![Effect::AddAbilityCounter {
            card_id: *card_id,
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Voidwalk,
                expires_on_effect: Some(EffectQuery::EnterZone {
                    card: card_id.into(),
                    zone: ZoneQuery::any_site(None, None),
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PlanarGate::NAME, |owner_id: PlayerId| {
    Box::new(PlanarGate::new(owner_id))
});
