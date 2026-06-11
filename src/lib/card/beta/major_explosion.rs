use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MajorExplosion {
    card_base: CardBase,
}

impl MajorExplosion {
    pub const NAME: &'static str = "Major Explosion";
    pub const DESCRIPTION: &'static str = "Target a location up to two steps away.\r \r Deal damage to each unit at locations in the area of effect:";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(7, "FF"),
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
impl Card for MajorExplosion {
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
impl Magic for MajorExplosion {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let locations = caster.get_locations_within_steps(state, 2);
        let prompt = "Pick a location to center the explosion";
        let location = pick_location_source(
            self.get_owner_id(),
            &locations,
            state,
            false,
            prompt,
            Some(*self.get_id()),
        )
        .await?;
        let location_dmg: Vec<(Option<Location>, u16)> = vec![
            (Some(location.clone()), 7),
            (location.steps_in_direction(&Direction::Up, 1), 5),
            (location.steps_in_direction(&Direction::Down, 1), 5),
            (location.steps_in_direction(&Direction::Left, 1), 5),
            (location.steps_in_direction(&Direction::Right, 1), 5),
            (location.steps_in_direction(&Direction::TopLeft, 1), 3),
            (location.steps_in_direction(&Direction::TopRight, 1), 3),
            (location.steps_in_direction(&Direction::BottomLeft, 1), 3),
            (location.steps_in_direction(&Direction::BottomRight, 1), 3),
        ];
        let mut effects = vec![];
        for (location, dmg) in location_dmg {
            if let Some(location) = location {
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MajorExplosion::NAME, |owner_id: PlayerId| {
        Box::new(MajorExplosion::new(owner_id))
    });
