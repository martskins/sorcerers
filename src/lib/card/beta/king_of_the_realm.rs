use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
    state::{CardMatcher, ContinousEffect, State},
};

#[derive(Debug, Clone)]
pub struct KingOfTheRealm {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl KingOfTheRealm {
    pub const NAME: &'static str = "King of the Realm";

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
                cost: Cost::new(7, "EEE"),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for KingOfTheRealm {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinousEffect>> {
        Ok(vec![
            ContinousEffect::ModifyPower {
                power_diff: 1,
                affected_cards: CardMatcher {
                    minion_types: Some(vec![MinionType::Mortal]),
                    not_in_ids: Some(vec![self.get_id().clone()]),
                    in_zones: Some(Zone::all_realm()),
                    ..Default::default()
                },
            },
            ContinousEffect::ControllerOverride {
                controller_id: self.get_controller_id(state),
                affected_cards: CardMatcher {
                    minion_types: Some(vec![MinionType::Mortal]),
                    in_zones: Some(Zone::all_realm()),
                    ..Default::default()
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (KingOfTheRealm::NAME, |owner_id: PlayerId| {
    Box::new(KingOfTheRealm::new(owner_id))
});
