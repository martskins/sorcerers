use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WrathOfTheSea {
    card_base: CardBase,
}

impl WrathOfTheSea {
    pub const NAME: &'static str = "Wrath of the Sea";
    pub const DESCRIPTION: &'static str = "Flood all sites adjacent to a body of water this turn. Then submerge all minions and artifacts on water.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(7, "WW"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WrathOfTheSea {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for WrathOfTheSea {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let prompt = "Pick a body of water";
        let bodies_of_water = state
            .get_bodies_of_water()
            .into_iter()
            .map(|body| body.iter().map(Zone::from).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let picked_body = pick_zone_group_source(
            controller_id,
            &bodies_of_water,
            state,
            false,
            prompt,
            Some(*self.get_id()),
        )
        .await?;
        let picked_body_locations = picked_body
            .iter()
            .filter_map(Zone::location)
            .cloned()
            .collect::<Vec<_>>();
        let sites = CardQuery::new().adjacent_to_locations(&picked_body_locations);
        let other_water_sites = CardQuery::new().water_sites().all(state);
        let zones: Vec<Zone> = sites
            .all(state)
            .iter()
            .chain(&other_water_sites)
            .map(|site| state.get_card(site).get_zone().clone())
            .collect();
        let minions_and_artifacts = CardQuery::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .in_zones(&zones)
            .all(state);
        let mut effects = minions_and_artifacts
            .into_iter()
            .map(|card_id| Effect::SetCardRegion {
                card_id,
                destination: Region::Underwater,
                tap: false,
            })
            .collect::<Vec<Effect>>();
        effects.push(Effect::AddTemporaryEffect {
            effect: TemporaryEffect::GrantAbility {
                ability: Ability::Flooded,
                affected_cards: sites,
                expires_on_effect: EffectQuery::TurnEnd { player_id: None },
            },
        });
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WrathOfTheSea::NAME, |owner_id: PlayerId| {
        Box::new(WrathOfTheSea::new(owner_id))
    });
