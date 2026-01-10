use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, SiteBase, SiteType, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct RimlandNomads {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl RimlandNomads {
    pub const NAME: &'static str = "Rimland Nomads";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                modifiers: vec![Ability::Movement(1)],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "F"),
                plane: Plane::Air,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Card for RimlandNomads {
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

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> anyhow::Result<Vec<Effect>> {
        let dealer = state.get_card(from);
        let dealer_is_desert = dealer
            .get_site_base()
            .unwrap_or(&SiteBase::default())
            .types
            .contains(&SiteType::Desert);
        if dealer_is_desert {
            return Ok(vec![]);
        }

        self.base_take_damage(state, from, damage)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (RimlandNomads::NAME, |owner_id: PlayerId| {
    Box::new(RimlandNomads::new(owner_id))
});
