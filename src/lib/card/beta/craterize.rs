use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Craterize {
    card_base: CardBase,
}

impl Craterize {
    pub const NAME: &'static str = "Craterize";
    pub const DESCRIPTION: &'static str = "As an additional cost to cast Craterize, discard a site. Destroy target site and deal damage to each unit above or below a site in the area of effect:";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::single(
                    Cost::new(8, "EE").with_additional(AdditionalCost::discard(
                        CardQuery::new()
                            .in_zone(&Zone::Hand)
                            .sites()
                            .controlled_by(&owner_id),
                    )),
                ),
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
impl Card for Craterize {
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
impl Magic for Craterize {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let Some(picked_site_id) = CardQuery::new()
            .sites()
            .with_prompt("Pick a site to destroy")
            .with_source_card(*self.get_id())
            .pick(&self.get_controller_id(state), state)
            .await?
        else {
            return Ok(vec![]);
        };
        let picked_site = state.get_card(&picked_site_id);

        let mut effects = vec![Effect::BuryCard {
            card_id: picked_site_id,
        }];

        let picked_location = picked_site.get_location();
        let Some(picked_square) = picked_location.square() else {
            return Ok(effects);
        };
        let square_then = |square: u8, first: &Direction, second: &Direction| {
            Location::square_after_steps_in_direction(square, first, 1, state, Some(caster_id))
                .and_then(|square| {
                    Location::square_after_steps_in_direction(
                        square,
                        second,
                        1,
                        state,
                        Some(caster_id),
                    )
                })
        };
        // Damage Pattern:
        // -------------------------------
        // |  1  |  2  |  4  |  2  |  1  |
        // -------------------------------
        // |  2  |  4  |  7  |  4  |  2  |
        // -------------------------------
        // |  4  |  7  | 10  |  7  |  4  |
        // -------------------------------
        // |  2  |  4  |  7  |  4  |  2  |
        // -------------------------------
        // |  1  |  2  |  4  |  2  |  1  |
        // -------------------------------
        #[rustfmt::skip]
        let square_damage = vec![
            (Some(picked_square), 10),
            (picked_location.square_in_direction(&Direction::Up, 1, state, Some(caster_id)), 7),
            (picked_location.square_in_direction(&Direction::Up, 2, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::Down, 1, state, Some(caster_id)), 7),
            (picked_location.square_in_direction(&Direction::Down, 2, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::Right, 1, state, Some(caster_id)), 7),
            (picked_location.square_in_direction(&Direction::Right, 2, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::Left, 1, state, Some(caster_id)), 7),
            (picked_location.square_in_direction(&Direction::Left, 2, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::TopLeft, 1, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::TopLeft, 2, state, Some(caster_id)), 1),
            (picked_location.square_in_direction(&Direction::TopRight, 1, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::TopRight, 2, state, Some(caster_id)), 1),
            (picked_location.square_in_direction(&Direction::BottomLeft, 1, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::BottomLeft, 2, state, Some(caster_id)), 1),
            (picked_location.square_in_direction(&Direction::BottomRight, 1, state, Some(caster_id)), 4),
            (picked_location.square_in_direction(&Direction::BottomRight, 2, state, Some(caster_id)), 1),
            (square_then(picked_square, &Direction::TopLeft, &Direction::Up), 2),
            (square_then(picked_square, &Direction::TopLeft, &Direction::Left), 2),
            (square_then(picked_square, &Direction::TopRight, &Direction::Up), 2),
            (square_then(picked_square, &Direction::TopRight, &Direction::Right), 2),
            (square_then(picked_square, &Direction::BottomLeft, &Direction::Up), 2),
            (square_then(picked_square, &Direction::BottomLeft, &Direction::Left), 2),
            (square_then(picked_square, &Direction::BottomRight, &Direction::Up), 2),
            (square_then(picked_square, &Direction::BottomRight, &Direction::Right), 2),
        ];

        for (square, damage) in square_damage {
            if let Some(square) = square {
                let surface_location = Location::Square(square, Region::Surface);
                if surface_location.get_site(state).is_none() {
                    continue;
                }

                for location in [
                    surface_location,
                    Location::Square(square, Region::Underground),
                    Location::Square(square, Region::Underwater),
                ] {
                    let units = CardQuery::new().units().in_location(location).all(state);
                    for unit_id in units {
                        effects.push(Effect::TakeDamage {
                            card_id: unit_id,
                            from: *self.get_id(),
                            damage: Damage::basic(damage),
                        });
                    }
                }
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Craterize::NAME, |owner_id: PlayerId| {
    Box::new(Craterize::new(owner_id))
});
