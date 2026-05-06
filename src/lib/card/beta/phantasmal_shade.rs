use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Damage, Edition, MinionType, Rarity,
        Region, UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct PhantasmalShade {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PhantasmalShade {
    pub const NAME: &'static str = "Phantasmal Shade";
    pub const DESCRIPTION: &'static str = "When Phantasmal Shade is struck, destroy it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Voidwalk, Ability::Stealth],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "AA"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for PhantasmalShade {
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
        _state: &State,
        _from: &uuid::Uuid,
        damage: Damage,
    ) -> anyhow::Result<Vec<Effect>> {
        if !damage.is_strike {
            return Ok(vec![]);
        }

        Ok(vec![Effect::BuryCard {
            card_id: *self.get_id(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PhantasmalShade::NAME, |owner_id: PlayerId| {
        Box::new(PhantasmalShade::new(owner_id))
    });
