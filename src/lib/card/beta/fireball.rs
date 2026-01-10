use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Fireball {
    pub card_base: CardBase,
}

impl Fireball {
    pub const NAME: &'static str = "Fireball";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "FF"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Fireball {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let prompt = "Fireball: Pick a direction to cast the spell:";
        let direction = pick_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS, state, prompt).await?;
        Ok(vec![Effect::ShootProjectile {
            player_id: self.get_owner_id().clone(),
            shooter: caster.get_id().clone(),
            from_zone: caster.get_zone().clone(),
            direction: direction.clone(),
            damage: 4,
            piercing: false,
            splash_damage: Some(2),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Fireball::NAME, |owner_id: PlayerId| Box::new(Fireball::new(owner_id)));
