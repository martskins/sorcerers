use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ColickyDragonettes {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl ColickyDragonettes {
    pub const NAME: &'static str = "Colicky Dragonettes";
    pub const DESCRIPTION: &'static str =
        "At the end of your turn, Colicky Dragonettes shoot a projectile. It deals 1 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Dragon],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
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
impl Card for ColickyDragonettes {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let is_current_player = &state.current_player == self.get_owner_id();
        if !is_current_player {
            return Ok(vec![]);
        }

        let prompt = "Colicky Dragonettes: Choose a direction to shoot a projectile";
        let direction =
            pick_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS, state, prompt).await?;
        Ok(vec![Effect::ShootProjectile {
            id: uuid::Uuid::new_v4(),
            player_id: *self.get_owner_id(),
            shooter: *self.get_id(),
            from_zone: self.get_zone().clone(),
            direction,
            damage: 1,
            piercing: false,
            splash_damage: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ColickyDragonettes::NAME, |owner_id: PlayerId| {
        Box::new(ColickyDragonettes::new(owner_id))
    });
