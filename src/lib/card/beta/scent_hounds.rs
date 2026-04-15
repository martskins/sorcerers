use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct ScentHounds {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl ScentHounds {
    pub const NAME: &'static str = "Scent Hounds";
    pub const DESCRIPTION: &'static str = "Nearby enemies permanently lose Stealth.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
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
                costs: Costs::basic(2, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ScentHounds {
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

    fn area_effects(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let opponent_id = state.get_opponent_id(&self.get_controller_id(state))?;
        let effects = CardQuery::new()
            .units()
            .near_to(self.get_zone())
            .controlled_by(&opponent_id)
            .with_abilities(vec![Ability::Stealth])
            .all(state)
            .into_iter()
            .map(|card_id| Effect::RemoveAbility {
                card_id: card_id,
                modifier: Ability::Stealth,
            })
            .collect();

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (ScentHounds::NAME, |owner_id: PlayerId| {
        Box::new(ScentHounds::new(owner_id))
    });
