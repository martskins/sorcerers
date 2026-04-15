use crate::{
    card::{
        AdditionalCost, Card, CardBase, Cost, Costs, Edition, Rarity, ResourceProvider, Site,
        SiteBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct UseAbility;

#[async_trait::async_trait]
impl ActivatedAbility for UseAbility {
    fn get_name(&self) -> String {
        "Use Vesuvius Ability".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::thresholds_only("FFF").with_additional(AdditionalCost::sacrifice(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let site_ids = CardQuery::new().sites().near_to(card.get_zone()).all(state);
        let mut effects = vec![];
        for site_id in site_ids {
            let site = state.get_card(&site_id);
            let units = state.get_units_in_zone(site.get_zone());
            for unit in units {
                effects.push(Effect::take_damage(unit.get_id(), card.get_id(), 3));
            }
        }
        Ok(effects)
    }
}

impl Site for Vesuvius {}

#[derive(Debug, Clone)]
pub struct Vesuvius {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Vesuvius {
    pub const NAME: &'static str = "Vesuvius";
    pub const DESCRIPTION: &'static str =
        "(F)(F)(F) — Sacrifice Vesuvius → Each unit occupying nearby sites takes 3 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Card for Vesuvius {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(UseAbility)])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Vesuvius::NAME, |owner_id: PlayerId| {
        Box::new(Vesuvius::new(owner_id))
    });
