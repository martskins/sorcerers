use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct SeirawanHydra {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SeirawanHydra {
    pub const NAME: &'static str = "Seirawan Hydra";
    pub const DESCRIPTION: &'static str = "Immediately heals from damage that doesn't kill it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                types: vec![MinionType::Monster],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "W"),
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
impl Card for SeirawanHydra {
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
        let damage_before = self.get_damage_taken()?;
        let mut effects = self.base_take_damage(state, from, damage, is_ranged)?;
        let damage_after = self.get_damage_taken()?;

        let survived = !effects
            .iter()
            .any(|effect| matches!(effect, Effect::KillMinion { card_id, .. } if *card_id == *self.get_id()));
        if survived && damage_after > damage_before {
            effects.push(Effect::Heal {
                card_id: *self.get_id(),
                amount: damage_after - damage_before,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SeirawanHydra::NAME, |owner_id: PlayerId| {
        Box::new(SeirawanHydra::new(owner_id))
    });
