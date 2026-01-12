use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Site, SiteBase, Zone},
    effect::{Effect, TokenType},
    game::{ActivatedAbility, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
struct UseAbility;

#[async_trait::async_trait]
impl ActivatedAbility for UseAbility {
    fn get_name(&self) -> &str {
        "Use Vesuvius Ability"
    }

    async fn on_select(&self, card_id: &uuid::Uuid, _: &PlayerId, state: &State) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let site_ids: Vec<uuid::Uuid> = card
            .get_zone()
            .get_nearby_sites(state, None)
            .iter()
            .map(|c| c.get_id().clone())
            .collect();
        let mut effects = vec![
            Effect::BuryCard {
                card_id: card.get_id().clone(),
                from: card.get_zone().clone(),
            },
            Effect::SummonToken {
                player_id: card.get_controller_id(state).clone(),
                token_type: TokenType::Rubble,
                zone: card.get_zone().clone(),
            },
        ];
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
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Vesuvius {
    pub const NAME: &'static str = "Vesuvius";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                plane: Plane::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Card for Vesuvius {
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

    fn get_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(UseAbility)])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Vesuvius::NAME, |owner_id: PlayerId| Box::new(Vesuvius::new(owner_id)));
