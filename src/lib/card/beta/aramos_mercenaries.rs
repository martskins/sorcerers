use crate::{
    card::{
        AdditionalCost, Card, CardBase, Cost, CostType, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    game::PlayerId,
    query::CardQuery,
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct AramosMercenaries {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AramosMercenaries {
    pub const NAME: &'static str = "Aramos Mercenaries";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "FF"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }

    fn discard_cost(&self, state: &State) -> anyhow::Result<Cost> {
        let controller_id = self.get_controller_id(state);
        let hand_cards = CardMatcher::new()
            .controller_id(&controller_id)
            .in_zone(&Zone::Hand)
            .resolve_ids(state);

        Ok(Cost {
            label: Some("Discard a random card".to_string()),
            mana: 0,
            thresholds: self.get_base().cost.thresholds.clone(),
            additional: vec![AdditionalCost::Discard {
                card: CardQuery::RandomTarget {
                    id: uuid::Uuid::new_v4(),
                    possible_targets: hand_cards,
                },
            }],
            cost_type: CostType::AlternativeCost,
        })
    }
}

#[async_trait::async_trait]
impl Card for AramosMercenaries {
    fn get_name(&self) -> &str {
        Self::NAME
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

    fn get_costs(&self, state: &State) -> anyhow::Result<Costs> {
        let base_cost = self.get_base().cost.clone();
        Ok(Costs::new(vec![base_cost, self.discard_cost(state)?]))
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AramosMercenaries::NAME, |owner_id: PlayerId| {
    Box::new(AramosMercenaries::new(owner_id))
});
