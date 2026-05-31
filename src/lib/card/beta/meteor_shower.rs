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

    fn sites_that_share_no_borders_with(state: &State, selected_zones: &[Zone]) -> Vec<CardId> {
        CardQuery::new()
            .sites()
            .in_play()
            .all(state)
            .into_iter()
            .filter(|site_id| {
                let zone = state.get_card(site_id).get_zone();
                selected_zones
                    .iter()
                    .all(|selected| zone != selected && !zone.is_adjacent(selected))
            })
            .collect()
    }

    fn large_impact(zone: &Zone) -> Vec<(Option<Zone>, u16)> {
        vec![
            (Some(zone.clone()), 7),
            (zone.zone_in_direction(&Direction::Up, 1), 5),
            (zone.zone_in_direction(&Direction::Down, 1), 5),
            (zone.zone_in_direction(&Direction::Left, 1), 5),
            (zone.zone_in_direction(&Direction::Right, 1), 5),
            (zone.zone_in_direction(&Direction::TopLeft, 1), 3),
            (zone.zone_in_direction(&Direction::TopRight, 1), 3),
            (zone.zone_in_direction(&Direction::BottomLeft, 1), 3),
            (zone.zone_in_direction(&Direction::BottomRight, 1), 3),
        ]
    }

    fn medium_impact(zone: &Zone) -> Vec<(Option<Zone>, u16)> {
        vec![
            (Some(zone.clone()), 4),
            (zone.zone_in_direction(&Direction::Up, 1), 2),
            (zone.zone_in_direction(&Direction::Down, 1), 2),
            (zone.zone_in_direction(&Direction::Left, 1), 2),
            (zone.zone_in_direction(&Direction::Right, 1), 2),
        ]
    }

    fn small_impact(zone: &Zone) -> Vec<(Option<Zone>, u16)> {
        vec![(Some(zone.clone()), 3)]
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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let first_site_id = pick_card(
            &controller_id,
            &Self::sites_that_share_no_borders_with(state, &[]),
            state,
            "Meteor Shower: Pick the center site for the 7-damage impact",
        )
        .await?;
        let first_zone = state.get_card(&first_site_id).get_zone().clone();

        let second_site_id = pick_card(
            &controller_id,
            &Self::sites_that_share_no_borders_with(state, std::slice::from_ref(&first_zone)),
            state,
            "Meteor Shower: Pick the center site for the 4-damage impact",
        )
        .await?;
        let second_zone = state.get_card(&second_site_id).get_zone().clone();

        let third_site_id = pick_card(
            &controller_id,
            &Self::sites_that_share_no_borders_with(
                state,
                &[first_zone.clone(), second_zone.clone()],
            ),
            state,
            "Meteor Shower: Pick the site for the 3-damage impact",
        )
        .await?;
        let third_zone = state.get_card(&third_site_id).get_zone().clone();

        let impacts = vec![
            Self::large_impact(&first_zone),
            Self::medium_impact(&second_zone),
            Self::small_impact(&third_zone),
        ];

        let mut effects = vec![];
        for (zone, damage) in impacts.into_iter().flatten() {
            let Some(zone) = zone else {
                continue;
            };
            if zone.get_site(state).is_none() {
                continue;
            }

            for unit_id in CardQuery::new().units().in_zone(&zone).all(state) {
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
