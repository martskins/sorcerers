use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Rubble, Site, SiteBase, Zone},
    effect::Effect,
    game::{CardAction, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
enum VesuviusAction {
    UseAbility,
}

#[async_trait::async_trait]
impl CardAction for VesuviusAction {
    fn get_name(&self) -> &str {
        match self {
            VesuviusAction::UseAbility => "Use Vesuvius Ability",
        }
    }

    async fn on_select(&self, card_id: &uuid::Uuid, _: &PlayerId, state: &State) -> anyhow::Result<Vec<Effect>> {
        match self {
            VesuviusAction::UseAbility => {
                let card = state.get_card(card_id);
                let site_ids: Vec<uuid::Uuid> = card
                    .get_zone()
                    .get_nearby_sites(state, None)
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect();
                let rubble = Rubble::new(card.get_owner_id().clone());
                let rubble_id = rubble.get_id().clone();
                let mut effects = vec![
                    Effect::bury_card(card.get_id(), card.get_zone()),
                    Effect::AddCard { card: Box::new(rubble) },
                    Effect::play_card(card.get_owner_id(), &rubble_id, card.get_zone()),
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
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
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

    fn get_actions(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn CardAction>>> {
        Ok(vec![Box::new(VesuviusAction::UseAbility)])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}
