use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WayfaringPilgrim {
    unit_base: UnitBase,
    card_base: CardBase,
    corners_visited: Vec<u8>,
}

impl WayfaringPilgrim {
    pub const NAME: &'static str = "Wayfaring Pilgrim";
    pub const DESCRIPTION: &'static str = "Whenever Wayfaring Pilgrim enters each corner of the realm for the first time, you may draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
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
            corners_visited: Vec::new(),
        }
    }
}

#[async_trait::async_trait]
impl Card for WayfaringPilgrim {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(corners_visited) = data.downcast_ref::<Vec<u8>>() {
            self.corners_visited = corners_visited.clone();
        }

        Ok(())
    }

    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let corner_squares = [1, 5, 16, 20];
        let corners = corner_squares
            .into_iter()
            .flat_map(|sq| {
                vec![
                    Location::Square(sq, Region::Surface),
                    Location::Square(sq, Region::Underwater),
                    Location::Square(sq, Region::Underground),
                    Location::Square(sq, Region::Void),
                ]
            })
            .map(Zone::Location)
            .collect();

        let mut corners_visited = self.corners_visited.clone();
        let Some(square) = self.get_zone().get_square() else {
            return Ok(vec![]);
        };

        if corners_visited.contains(&square) {
            return Ok(vec![]);
        }

        corners_visited.push(square);
        Ok(vec![Hook {
            trigger: EffectQuery::EnterZone {
                card: self.get_id().into(),
                zone: ZoneQuery::from_options(corners, None),
                from: None,
            },
            timing: HookTiming::After,
            action: HookAction::Effects(vec![
                Effect::DrawCard {
                    player_id: self.get_controller_id(state),
                    count: 1,
                    kind: DrawKind::Choice,
                },
                Effect::SetCardData {
                    card_id: *self.get_id(),
                    data: std::sync::Arc::new(corners_visited),
                },
            ]),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WayfaringPilgrim::NAME, |owner_id: PlayerId| {
        Box::new(WayfaringPilgrim::new(owner_id))
    });
