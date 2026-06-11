use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ConeOfFlame {
    card_base: CardBase,
}

impl ConeOfFlame {
    pub const NAME: &'static str = "Cone of Flame";
    pub const DESCRIPTION: &'static str = "Choose a direction from the caster. Deal damage to each unit at a location in the area of effect:";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
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
impl Card for ConeOfFlame {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for ConeOfFlame {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let prompt = "Pick a direction to cast the spell";
        let dir = pick_direction_source(
            self.get_owner_id(),
            &CARDINAL_DIRECTIONS,
            state,
            prompt,
            Some(*caster_id),
        )
        .await?;
        let caster = state.get_card(caster_id);
        let location = caster.get_location();
        let region = location.region().clone();
        let square_dmg = vec![
            (location.square_in_direction(&dir, 1), 5),
            (location.square_in_direction(&dir, 2), 3),
            (location.square_in_direction(&dir, 3), 1),
            (location.square_in_direction(&dir.rotate(1)?, 1), 3),
            (location.square_in_direction(&dir.rotate(1)?, 2), 1),
            (location.square_in_direction(&dir.rotate(7)?, 1), 3),
            (location.square_in_direction(&dir.rotate(7)?, 2), 1),
        ];

        let mut effects = vec![];
        for (square, dmg) in square_dmg {
            if let Some(square) = square {
                let location = Location::Square(square, region.clone());
                let units = CardQuery::new().units().in_location(location).all(state);
                for unit in units {
                    effects.push(Effect::TakeDamage {
                        card_id: unit,
                        from: *self.get_id(),
                        damage: Damage::basic(dmg),
                    });
                }
            }
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (ConeOfFlame::NAME, |owner_id: PlayerId| {
    Box::new(ConeOfFlame::new(owner_id))
});
