use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, SiteBase, SiteType,
        UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct RimlandNomads {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RimlandNomads {
    pub const NAME: &'static str = "Rimland Nomads";
    pub const DESCRIPTION: &'static str = "Movement +1\r \r Takes no damage from Deserts.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Movement(1)],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Card for RimlandNomads {
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
    ) -> anyhow::Result<Vec<Effect>> {
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (RimlandNomads::NAME, |owner_id: PlayerId| {
        Box::new(RimlandNomads::new(owner_id))
    });
