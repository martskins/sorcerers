use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, Plane, Rarity, Site, SiteBase, Zone},
    effect::{AbilityCounter, Effect},
    game::{PlayerId, Thresholds},
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct Quagmire {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Quagmire {
    pub const NAME: &'static str = "Quagmire";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                types: vec![],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for Quagmire {}

#[async_trait::async_trait]
impl Card for Quagmire {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let effects = self
            .get_zone()
            .get_nearby_sites(state, None)
            .iter()
            .map(|s| s.get_zone().get_units(state, None))
            .flatten()
            .map(|u| Effect::AddAbilityCounter {
                card_id: u.get_id().clone(),
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    modifier: Ability::Immobile,
                    expires_on_effect: Some(EffectQuery::TurnStart {
                        player_id: Some(self.get_controller_id(state).clone()),
                    }),
                },
            })
            .collect();

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Quagmire::NAME, |owner_id: PlayerId| Box::new(Quagmire::new(owner_id)));
