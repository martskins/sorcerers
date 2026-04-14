use crate::{
    card::{
        Card, CardBase, Cost, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds},
    query::EffectQuery,
    state::{CardQuery, State, TemporaryEffect},
};

#[derive(Debug, Clone)]
struct FloodAdjacentSite;

#[async_trait::async_trait]
impl ActivatedAbility for FloodAdjacentSite {
    fn get_name(&self) -> String {
        "Flood Adjacent Site".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        Ok(vec![
            Effect::AddTemporaryEffect {
                effect: TemporaryEffect::FloodSites {
                    affected_sites: CardQuery::new()
                        .sites()
                        .adjacent_to(card.get_zone())
                        .with_prompt("Floodplain: Pick an adjacent site to flood"),
                    expires_on_effect: EffectQuery::TurnEnd { player_id: None },
                },
            },
            Effect::SetCardData {
                card_id: card_id.clone(),
                data: Box::new(state.turns),
            },
        ])
    }

    fn get_cost(&self, _card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::ZERO)
    }

    fn can_activate(
        &self,
        _card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct Floodplain {
    pub site_base: SiteBase,
    pub card_base: CardBase,
    last_activation_on_turn: Option<usize>,
}

impl Floodplain {
    pub const NAME: &'static str = "Floodplain";
    pub const DESCRIPTION: &'static str =
        "Once on your turn, you may flood an adjacent site this turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
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
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            last_activation_on_turn: None,
        }
    }
}

impl Site for Floodplain {}

#[async_trait::async_trait]
impl Card for Floodplain {
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

    fn get_additional_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if let Some(last_activation) = self.last_activation_on_turn {
            if last_activation == state.turns {
                return Ok(vec![]);
            }
        }

        if state.current_player != self.card_base.controller_id {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(FloodAdjacentSite)])
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(last_activation) = data.downcast_ref::<usize>() {
            self.last_activation_on_turn = Some(*last_activation);
        }

        Ok(())
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Floodplain::NAME, |owner_id: PlayerId| {
        Box::new(Floodplain::new(owner_id))
    });
