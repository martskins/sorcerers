use crate::{
    card::{
        AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity,
        ResourceProvider, Site, SiteBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds, pick_card},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct FreeCityAttack;

#[async_trait::async_trait]
impl ActivatedAbility for FreeCityAttack {
    fn get_name(&self) -> String {
        "Tap to attack or defend against enemies here".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    fn can_activate(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        let enemies_here = CardQuery::new()
            .units()
            .in_zone(card.get_zone())
            .all(state)
            .into_iter()
            .filter(|id| &state.get_card(id).get_controller_id(state) != player_id)
            .count();
        Ok(enemies_here > 0)
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let enemies_here: Vec<uuid::Uuid> = CardQuery::new()
            .units()
            .in_zone(card.get_zone())
            .all(state)
            .into_iter()
            .filter(|id| &state.get_card(id).get_controller_id(state) != player_id)
            .collect();

        if enemies_here.is_empty() {
            return Ok(vec![]);
        }

        let target = pick_card(
            player_id,
            &enemies_here,
            state,
            "Free City: pick an enemy to attack",
        )
        .await?;

        Ok(vec![
            Effect::TakeDamage {
                card_id: target,
                from: *card_id,
                damage: 3,
                is_strike: false,
            },
            Effect::SetCardData {
                card_id: *card_id,
                data: Box::new(true),
            },
        ])
    }
}

// TODO: This implementation is not really correct. The site is not able to defend now, but
// we cannot add a unit base to it, as it would become a unit if we did.
#[derive(Debug, Clone)]
pub struct FreeCity {
    site_base: SiteBase,
    card_base: CardBase,
    used_ability: bool,
}

impl FreeCity {
    pub const NAME: &'static str = "Free City";
    pub const DESCRIPTION: &'static str =
        "Once per turn, may attack or defend against enemy units here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 3,
                provided_thresholds: Thresholds::new(),
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
            used_ability: false,
        }
    }
}

impl Site for FreeCity {}

#[async_trait::async_trait]
impl Card for FreeCity {
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

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(site_data) = data.downcast_ref::<bool>() {
            self.used_ability = *site_data;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for FreeCity"))
        }
    }

    async fn on_turn_start(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::SetCardData {
            card_id: *self.get_id(),
            data: Box::new(false),
        }])
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if self.used_ability {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(FreeCityAttack)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FreeCity::NAME, |owner_id: PlayerId| {
    Box::new(FreeCity::new(owner_id))
});
