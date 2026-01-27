use crate::{
    card::{AdditionalCost, Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Direction, PlayerId, pick_card},
    query::{CardQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Craterize {
    pub card_base: CardBase,
}

impl Craterize {
    pub const NAME: &'static str = "Craterize";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(8, "EE"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Craterize {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_additional_costs(&self, state: &State) -> anyhow::Result<Vec<AdditionalCost>> {
        Ok(vec![
            AdditionalCost::Discard {
                card: CardQuery::InZone {
                    id: uuid::Uuid::new_v4(),
                    zone: Zone::Hand,
                    card_types: Some(vec![CardType::Site]),
                    regions: None,
                    owner: Some(self.get_controller_id(state)),
                    prompt: Some("Craterize: Discard a site from your hand".to_string()),
                    tapped: None,
                },
            },
            AdditionalCost::Discard {
                card: CardQuery::InZone {
                    id: uuid::Uuid::new_v4(),
                    zone: Zone::Hand,
                    card_types: Some(vec![CardType::Site]),
                    regions: None,
                    owner: Some(self.get_controller_id(state)),
                    prompt: Some("Craterize: Discard a site from your hand".to_string()),
                    tapped: None,
                },
            },
        ])
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let sites = state
            .cards
            .iter()
            .filter(|c| c.is_site())
            .filter(|c| c.get_zone().is_in_play())
            .map(|c| c.get_id())
            .cloned()
            .collect::<Vec<_>>();
        let picked_site_id = pick_card(
            &self.get_controller_id(state),
            &sites,
            state,
            "Craterize: Pick a site to destroy",
        )
        .await?;
        let picked_site = state.get_card(&picked_site_id);

        let mut effects = vec![Effect::BuryCard {
            card_id: picked_site_id.clone(),
        }];

        let picked_zone = picked_site.get_zone();
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
        let zone_damage = vec![
            (Some(picked_zone.clone()), 10),
            (picked_zone.zone_in_direction(&Direction::Up, 1), 7),
            (picked_zone.zone_in_direction(&Direction::Up, 2), 4),
            (picked_zone.zone_in_direction(&Direction::Down, 1), 7),
            (picked_zone.zone_in_direction(&Direction::Down, 2), 4),
            (picked_zone.zone_in_direction(&Direction::Right, 1), 7),
            (picked_zone.zone_in_direction(&Direction::Right, 2), 4),
            (picked_zone.zone_in_direction(&Direction::Left, 1), 7),
            (picked_zone.zone_in_direction(&Direction::Left, 2), 4),
            (picked_zone.zone_in_direction(&Direction::TopLeft, 1), 4),
            (picked_zone.zone_in_direction(&Direction::TopLeft, 2), 1),
            (picked_zone.zone_in_direction(&Direction::TopRight, 1), 4),
            (picked_zone.zone_in_direction(&Direction::TopRight, 2), 1),
            (picked_zone.zone_in_direction(&Direction::BottomLeft, 1), 4),
            (picked_zone.zone_in_direction(&Direction::BottomLeft, 2), 1),
            (picked_zone.zone_in_direction(&Direction::BottomRight, 1), 4),
            (picked_zone.zone_in_direction(&Direction::BottomRight, 2), 1),
            (picked_zone.zone_in_direction(&Direction::TopLeft, 1).and_then(|z| z.zone_in_direction(&Direction::Up, 1)), 2),
            (picked_zone.zone_in_direction(&Direction::TopLeft, 1).and_then(|z| z.zone_in_direction(&Direction::Left, 1)), 2),
            (picked_zone.zone_in_direction(&Direction::TopRight, 1).and_then(|z| z.zone_in_direction(&Direction::Up, 1)), 2),
            (picked_zone.zone_in_direction(&Direction::TopRight, 1).and_then(|z| z.zone_in_direction(&Direction::Right, 1)), 2),
            (picked_zone.zone_in_direction(&Direction::BottomLeft, 1).and_then(|z| z.zone_in_direction(&Direction::Up, 1)), 2),
            (picked_zone.zone_in_direction(&Direction::BottomLeft, 1).and_then(|z| z.zone_in_direction(&Direction::Left, 1)), 2),
            (picked_zone.zone_in_direction(&Direction::BottomRight, 1).and_then(|z| z.zone_in_direction(&Direction::Up, 1)), 2),
            (picked_zone.zone_in_direction(&Direction::BottomRight, 1).and_then(|z| z.zone_in_direction(&Direction::Right, 1)), 2),
        ];

        for (zone, damage) in zone_damage {
            if let Some(zone) = zone {
                if zone.get_site(state).is_none() {
                    continue;
                }

                effects.push(Effect::DealDamageAllUnitsInZone {
                    player_id: self.get_controller_id(state).clone(),
                    zone: ZoneQuery::Specific {
                        id: uuid::Uuid::new_v4(),
                        zone: zone,
                    },
                    from: caster_id.clone(),
                    damage: damage,
                });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Craterize::NAME, |owner_id: PlayerId| Box::new(Craterize::new(owner_id)));