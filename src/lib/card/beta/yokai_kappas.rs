use crate::{
    card::{
        AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity,
        Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct DiscardWaterSiteToUntap;

#[async_trait::async_trait]
impl ActivatedAbility for DiscardWaterSiteToUntap {
    fn get_name(&self) -> String {
        "Discard a water site → Untap".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::discard(
            CardQuery::new().water_sites().including_not_in_play().in_zone(&Zone::Hand),
        ))
        .with_additional(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::UntapCard { card_id: *card_id }])
    }
}

#[derive(Debug, Clone)]
pub struct YokaiKappas {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl YokaiKappas {
    pub const NAME: &'static str = "Yokai Kappas";
    pub const DESCRIPTION: &'static str =
        "Discard a water site → Untap Yokai Kappas. Use only once per turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
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
impl Card for YokaiKappas {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(DiscardWaterSiteToUntap)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (YokaiKappas::NAME, |owner_id: PlayerId| {
        Box::new(YokaiKappas::new(owner_id))
    });
