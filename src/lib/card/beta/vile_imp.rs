use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase,
        Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct VileImp {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl VileImp {
    pub const NAME: &'static str = "Vile Imp";
    pub const DESCRIPTION: &'static str =
        "Genesis → You may deal 2 damage to a target adjacent unit.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Demon],
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

#[async_trait::async_trait]
impl Card for VileImp {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let imp_id = *self.get_id();

        let Some(target_id) = CardQuery::new()
            .units()
            .adjacent_to(self.get_zone())
            .id_not_in(vec![imp_id])
            .with_prompt("Vile Imp: Optionally deal 2 damage to a target adjacent unit")
            .pick(&controller_id, state, true)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::TakeDamage {
            card_id: target_id,
            from: imp_id,
            damage: 2,
            is_strike: false,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (VileImp::NAME, |owner_id: PlayerId| {
    Box::new(VileImp::new(owner_id))
});
