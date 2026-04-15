use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct DealDamage;

#[async_trait::async_trait]
impl ActivatedAbility for DealDamage {
    fn get_name(&self) -> String {
        "Deal Damage".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let possible_targets = CardQuery::new()
            .randomised()
            .count(1)
            .in_zone(card.get_zone())
            .id_not_in(vec![card_id.clone()])
            .units();

        Ok(vec![Effect::DealDamageToTarget {
            from: card_id.clone(),
            damage: 3,
            player_id: card.get_controller_id(state).clone(),
            query: possible_targets,
        }])
    }
}

#[derive(Debug, Clone)]
pub struct NimbusJinn {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl NimbusJinn {
    pub const NAME: &'static str = "Nimbus Jinn";
    pub const DESCRIPTION: &'static str =
        "Airborne\r \r Discard a spell → Deal 3 damage to another random unit here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "AA"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for NimbusJinn {
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
        Ok(vec![Box::new(DealDamage)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (NimbusJinn::NAME, |owner_id: PlayerId| {
        Box::new(NimbusJinn::new(owner_id))
    });
