use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ColickyDragonettes {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl ColickyDragonettes {
    pub const NAME: &'static str = "Colicky Dragonettes";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Dragon],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "FF"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ColickyDragonettes {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let is_current_player = &state.current_player == self.get_owner_id();
        if !is_current_player {
            return Ok(vec![]);
        }

        let prompt = "Colicky Dragonettes: Choose a direction to shoot a projectile";
        let direction = pick_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS, state, prompt).await?;
        Ok(vec![Effect::ShootProjectile {
            id: uuid::Uuid::new_v4(),
            player_id: self.get_owner_id().clone(),
            shooter: self.get_id().clone(),
            from_zone: self.get_zone().clone(),
            direction,
            damage: 1,
            piercing: false,
            splash_damage: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (ColickyDragonettes::NAME, |owner_id: PlayerId| {
    Box::new(ColickyDragonettes::new(owner_id))
});
