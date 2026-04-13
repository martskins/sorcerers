use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Upwelling {
    pub card_base: CardBase,
}

impl Upwelling {
    pub const NAME: &'static str = "Upwelling";
    pub const DESCRIPTION: &'static str =
        "Target a nearby site. Return each artifact and minion there to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Upwelling {
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let Some(site_id) = CardQuery::new()
            .sites()
            .near_to(caster.get_zone())
            .with_prompt("Upwelling: Pick a site")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        let site = state.get_card(&site_id);
        let effects = CardQuery::new()
            .in_zone(site.get_zone())
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .all(state)
            .into_iter()
            .map(|card_id| Effect::SetCardZone {
                card_id,
                zone: Zone::Hand,
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Upwelling::NAME, |owner_id: PlayerId| Box::new(Upwelling::new(owner_id)));
