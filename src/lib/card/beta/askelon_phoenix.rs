use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::{Counter, Effect},
    game::{Element, PlayerId},
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct AskelonPhoenix {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl AskelonPhoenix {
    pub const NAME: &'static str = "Askelon Phoenix";
    pub const DESCRIPTION: &'static str = "Airborne\r \r If Askelon Phoenix would take fire damage, it gains +1 power this turn instead.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "FF"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Card for AskelonPhoenix {
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

    fn on_take_damage(
        &mut self,
        state: &State,
        from: &uuid::Uuid,
        damage: u16,
        is_ranged: bool,
    ) -> anyhow::Result<Vec<Effect>> {
        let attacker = state.get_card(from);
        if attacker.get_elements(state)?.contains(&Element::Fire) {
            return Ok(vec![Effect::AddCounter {
                card_id: *self.get_id(),
                counter: Counter::new(1, 1, Some(EffectQuery::TurnEnd { player_id: None })),
            }]);
        }

        let effects = self.base_take_damage(state, from, damage, is_ranged)?;
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AskelonPhoenix::NAME, |owner_id: PlayerId| {
        Box::new(AskelonPhoenix::new(owner_id))
    });
