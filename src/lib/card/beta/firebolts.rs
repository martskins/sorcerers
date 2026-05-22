use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Firebolts {
    card_base: CardBase,
}

impl Firebolts {
    pub const NAME: &'static str = "Firebolts";
    pub const DESCRIPTION: &'static str =
        "Shoot three projectiles in the same direction, one at a time. Each deals 1 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
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
impl Card for Firebolts {
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
            Some(*self.get_id()),
        )
        .await?;
        Ok(vec![
            Effect::ShootProjectile {
                id: uuid::Uuid::new_v4(),
                range: None,
                player_id: *self.get_owner_id(),
                shooter: *caster.get_id(),
                from_zone: caster.get_zone().clone(),
                direction: direction.clone(),
                damage: 1,
                ranged_strike: false,
                piercing: false,
                splash_damage: None,
            },
            Effect::ShootProjectile {
                id: uuid::Uuid::new_v4(),
                range: None,
                player_id: *self.get_owner_id(),
                shooter: *caster.get_id(),
                from_zone: caster.get_zone().clone(),
                direction: direction.clone(),
                damage: 1,
                ranged_strike: false,
                piercing: false,
                splash_damage: None,
            },
            Effect::ShootProjectile {
                id: uuid::Uuid::new_v4(),
                range: None,
                player_id: *self.get_owner_id(),
                shooter: *caster.get_id(),
                from_zone: caster.get_zone().clone(),
                direction,
                damage: 1,
                ranged_strike: false,
                piercing: false,
                splash_damage: None,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Firebolts::NAME, |owner_id: PlayerId| {
    Box::new(Firebolts::new(owner_id))
});
