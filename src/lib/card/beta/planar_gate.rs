use crate::{
    card::{Card, CardBase, Edition, Modifier, Plane, Rarity, Site, SiteBase, SiteType, Zone},
    effect::{Effect, ModifierCounter},
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
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Elite,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for PlanarGate {
    fn on_card_enter(&self, state: &State, card_id: &uuid::Uuid) -> Vec<Effect> {
        let card = state.get_card(card_id).unwrap();
        if !card.is_minion() {
            return vec![];
        }

        vec![Effect::AddModifier {
            card_id: card_id.clone(),
            counter: ModifierCounter {
                id: uuid::Uuid::new_v4(),
                modifier: Modifier::Voidwalk,
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

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}
