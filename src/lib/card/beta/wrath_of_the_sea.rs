use crate::{
    card::{Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone_group},
    query::EffectQuery,
    state::{CardMatcher, State, TemporaryEffect},
};

#[derive(Debug, Clone)]
pub struct WrathOfTheSea {
    pub card_base: CardBase,
}

impl WrathOfTheSea {
    pub const NAME: &'static str = "Wrath of the Sea";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(7, "WW"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WrathOfTheSea {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let prompt = "Wrath of the Sea: Pick a body of water";
        let bodies_of_water = state.get_bodies_of_water();
        let picked_body = pick_zone_group(controller_id, &bodies_of_water, state, false, prompt).await?;
        let sites = CardMatcher::new().adjacent_to_zones(&picked_body);
        let other_water_sites = CardMatcher::new().card_type(CardType::Site).resolve_ids(state);
        let zones: Vec<Zone> = sites
            .resolve_ids(state)
            .iter()
            .chain(&other_water_sites)
            .map(|site| state.get_card(site).get_zone().clone())
            .collect();
        let minions_and_artifacts = CardMatcher::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .in_zones(&zones)
            .resolve_ids(state);
        let mut effects = minions_and_artifacts
            .into_iter()
            .map(|card_id| Effect::SetCardRegion {
                card_id,
                region: Region::Underwater,
                tap: false,
            })
            .collect::<Vec<Effect>>();
        effects.push(Effect::AddTemporaryEffect {
            effect: TemporaryEffect::FloodSites {
                affected_sites: sites,
                expires_on_effect: EffectQuery::TurnEnd { player_id: None },
            },
        });
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (WrathOfTheSea::NAME, |owner_id: PlayerId| {
    Box::new(WrathOfTheSea::new(owner_id))
});