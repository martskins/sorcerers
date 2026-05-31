use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Fireball {
    card_base: CardBase,
}

impl Fireball {
    pub const NAME: &'static str = "Fireball";
    pub const DESCRIPTION: &'static str = "Shoot a projectile. It deals 4 damage on impact, and 2 damage to each other unit at that location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
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
impl Card for Fireball {
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let prompt = "Pick a direction to cast the spell";
        let direction = pick_direction_source(
            self.get_owner_id(),
            &CARDINAL_DIRECTIONS,
            state,
            prompt,
            Some(*caster_id),
        )
        .await?;
        Ok(vec![Effect::ShootProjectile {
            id: uuid::Uuid::new_v4(),
            range: None,
            player_id: *self.get_owner_id(),
            shooter: *caster.get_id(),
            from_zone: caster.get_zone().clone(),
            direction: direction.clone(),
            damage: 4,
            ranged_strike: false,
            piercing: false,
            splash_damage: Some(2),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Fireball::NAME, |owner_id: PlayerId| {
    Box::new(Fireball::new(owner_id))
});
