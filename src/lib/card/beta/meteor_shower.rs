use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MeteorShower {
    card_base: CardBase,
}

impl MeteorShower {
    pub const NAME: &'static str = "Meteor Shower";
    pub const DESCRIPTION: &'static str = "Target three sites that share no borders. Deal damage to each unit atop sites in each area of effect:
                    |2|
        3|5|3      2|4|2
        5|7|5       |2|
        3|5|3
               |3|";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(9, "FFF"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn sites_that_share_no_borders_with(
        state: &State,
        selected_locations: &[Location],
    ) -> Vec<CardId> {
        CardQuery::new()
            .sites()
            .in_play()
            .all(state)
            .into_iter()
            .filter(|site_id| {
                let location = state.get_card(site_id).get_location();
                selected_locations
                    .iter()
                    .all(|selected| location != selected && !location.is_adjacent(selected))
            })
            .collect()
    }

    fn large_impact(
        location: &Location,
        state: &State,
        caster_id: &CardId,
    ) -> Vec<(Option<Location>, u16)> {
        #[rustfmt::skip]
        let result = vec![
            (Some(location.clone()), 7),
            (location.steps_in_direction(&Direction::Up, 1, state, Some(caster_id)), 5),
            (location.steps_in_direction(&Direction::Down, 1, state, Some(caster_id)), 5),
            (location.steps_in_direction(&Direction::Left, 1, state, Some(caster_id)), 5),
            (location.steps_in_direction(&Direction::Right, 1, state, Some(caster_id)), 5),
            (location.steps_in_direction(&Direction::TopLeft, 1, state, Some(caster_id)), 3),
            (location.steps_in_direction(&Direction::TopRight, 1, state, Some(caster_id)), 3),
            (location.steps_in_direction(&Direction::BottomLeft, 1, state, Some(caster_id)), 3),
            (location.steps_in_direction(&Direction::BottomRight, 1, state, Some(caster_id)), 3),
        ];

        result
    }

    fn medium_impact(
        location: &Location,
        state: &State,
        caster_id: &CardId,
    ) -> Vec<(Option<Location>, u16)> {
        vec![
            (Some(location.clone()), 4),
            (
                location.steps_in_direction(&Direction::Up, 1, state, Some(caster_id)),
                2,
            ),
            (
                location.steps_in_direction(&Direction::Down, 1, state, Some(caster_id)),
                2,
            ),
            (
                location.steps_in_direction(&Direction::Left, 1, state, Some(caster_id)),
                2,
            ),
            (
                location.steps_in_direction(&Direction::Right, 1, state, Some(caster_id)),
                2,
            ),
        ]
    }

    fn small_impact(location: &Location) -> Vec<(Option<Location>, u16)> {
        vec![(Some(location.clone()), 3)]
    }
}

#[async_trait::async_trait]
impl Card for MeteorShower {
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
impl Magic for MeteorShower {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(first_site_id) =
            CardQuery::from_ids(Self::sites_that_share_no_borders_with(state, &[]))
                .with_prompt("Pick the center site for the 7-damage impact")
                .with_source_card(*self.get_id())
                .pick(&controller_id, state)
                .await?
        else {
            return Ok(vec![]);
        };
        let first_location = state.get_card(&first_site_id).get_location().clone();

        let Some(second_site_id) = CardQuery::from_ids(Self::sites_that_share_no_borders_with(
            state,
            std::slice::from_ref(&first_location),
        ))
        .with_prompt("Pick the center site for the 4-damage impact")
        .with_source_card(*self.get_id())
        .pick(&controller_id, state)
        .await?
        else {
            return Ok(vec![]);
        };
        let second_location = state.get_card(&second_site_id).get_location().clone();

        let Some(third_site_id) = CardQuery::from_ids(Self::sites_that_share_no_borders_with(
            state,
            &[first_location.clone(), second_location.clone()],
        ))
        .with_prompt("Pick the site for the 3-damage impact")
        .with_source_card(*self.get_id())
        .pick(&controller_id, state)
        .await?
        else {
            return Ok(vec![]);
        };
        let third_location = state.get_card(&third_site_id).get_location().clone();

        let impacts = vec![
            Self::large_impact(&first_location, state, caster_id),
            Self::medium_impact(&second_location, state, caster_id),
            Self::small_impact(&third_location),
        ];

        let mut effects = vec![];
        for (location, damage) in impacts.into_iter().flatten() {
            let Some(location) = location else {
                continue;
            };
            if location.get_site(state).is_none() {
                continue;
            }

            for unit_id in CardQuery::new().units().in_location(location).all(state) {
                effects.push(Effect::TakeDamage {
                    card_id: unit_id,
                    from: *self.get_id(),
                    damage: Damage::basic(damage),
                });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MeteorShower::NAME, |owner_id: PlayerId| {
    Box::new(MeteorShower::new(owner_id))
});
