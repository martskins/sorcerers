use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId},
    query::CardQuery,
    state::State,
};

#[derive(Debug, Clone)]
struct DealDamage;

#[async_trait::async_trait]
impl ActivatedAbility for DealDamage {
    fn get_name(&self) -> &str {
        todo!()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let units = state
            .get_units_in_zone(card.get_zone())
            .iter()
            .filter(|c| c.get_id() != card_id)
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        if units.len() == 0 {
            return Ok(vec![]);
        }

        Ok(vec![Effect::DealDamageToTarget {
            from: card_id.clone(),
            damage: 3,
            player_id: card.get_controller_id().clone(),
            query: CardQuery::RandomTarget {
                id: uuid::Uuid::new_v4(),
                possible_targets: units,
            },
        }])
    }
}

#[derive(Debug, Clone)]
pub struct NimbusJinn {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl NimbusJinn {
    pub const NAME: &'static str = "Nimbus Jinn";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Spirit],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(6, "AA"),
                plane: Plane::Air,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for NimbusJinn {
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

    fn get_activated_abilities(&self, state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        let mut activated_abilities = self.base_unit_activated_abilities(state)?;
        activated_abilities.push(Box::new(DealDamage));
        Ok(activated_abilities)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (NimbusJinn::NAME, |owner_id: PlayerId| {
    Box::new(NimbusJinn::new(owner_id))
});
