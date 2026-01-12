use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    effect::{Counter, Effect},
    game::{Element, PlayerId},
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct AskelonPhoenix {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AskelonPhoenix {
    pub const NAME: &'static str = "Askelon Phoenix";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(5, "FF"),
                plane: Plane::Air,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Card for AskelonPhoenix {
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

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u16) -> anyhow::Result<Vec<Effect>> {
        let attacker = state.get_card(from);
        if attacker.get_elements(state)?.contains(&Element::Fire) {
            return Ok(vec![Effect::AddCounter {
                card_id: self.get_id().clone(),
                counter: Counter::new(1, 1, Some(EffectQuery::TurnEnd { player_id: None })),
            }]);
        }

        let ub = self.get_unit_base_mut().unwrap();
        ub.damage += damage;

        let mut effects = vec![];
        if ub.damage >= self.get_toughness(state).unwrap_or(0) || attacker.has_modifier(state, &Ability::Lethal) {
            effects.push(Effect::BuryCard {
                card_id: self.get_id().clone(),
                from: self.get_zone().clone(),
            });
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AskelonPhoenix::NAME, |owner_id: PlayerId| {
    Box::new(AskelonPhoenix::new(owner_id))
});
