use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct KarkemishChimera {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl KarkemishChimera {
    pub const NAME: &'static str = "Karkemish Chimera";
    pub const DESCRIPTION: &'static str =
        "Can simultaneously attack up to three units at the same location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "FF"),
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
impl Card for KarkemishChimera {
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

    fn on_attack(&self, state: &State, defender_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let defender = state.get_card(defender_id);
        if !defender.is_unit() {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let power = self.get_power(state)?.unwrap_or_default();
        let attacker_id = *self.get_id();
        let mut effects = vec![];

        for extra_id in CardQuery::new()
            .units()
            .in_zone(defender.get_zone())
            .all(state)
            .into_iter()
            .filter(|id| id != defender_id)
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .take(2)
        {
            effects.push(Effect::TakeDamage {
                card_id: extra_id,
                from: attacker_id,
                damage: power,
                is_strike: false,
                is_ranged: false,
            });
            effects.extend(state.get_card(&extra_id).on_defend(state, &attacker_id)?);
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (KarkemishChimera::NAME, |owner_id: PlayerId| {
        Box::new(KarkemishChimera::new(owner_id))
    });
