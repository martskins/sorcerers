use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site,
        SiteBase, Zone,
    },
    effect::{AbilityCounter, Effect},
    game::{PlayerId, Thresholds},
    query::EffectQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Quagmire {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Quagmire {
    pub const NAME: &'static str = "Quagmire";
    pub const DESCRIPTION: &'static str =
        "Genesis → Until your next turn, units are Immobile while they occupy nearby sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let effects = CardQuery::new()
            .units()
            .near_to(self.get_zone())
            .all(state)
            .into_iter()
            .map(|card_id| Effect::AddAbilityCounter {
                card_id,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Immobile,
                    expires_on_effect: Some(EffectQuery::TurnStart {
                        player_id: Some(self.get_controller_id(state)),
                    }),
                },
            })
            .collect();

        Ok(effects)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Quagmire::NAME, |owner_id: PlayerId| {
    Box::new(Quagmire::new(owner_id))
});
