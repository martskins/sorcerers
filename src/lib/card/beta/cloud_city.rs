use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, Zone},
    effect::Effect,
    game::{CardAction, PlayerId, Thresholds, pick_zone},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
enum CloudCityAction {
    FlyToVoid,
}

#[async_trait::async_trait]
impl CardAction for CloudCityAction {
    fn get_name(&self) -> &str {
        match self {
            CloudCityAction::FlyToVoid => "Fly to nearby void",
        }
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            CloudCityAction::FlyToVoid => {
                let card = state.get_card(card_id);
                let nearby_voids: Vec<Zone> = card
                    .get_zone()
                    .get_nearby()
                    .iter()
                    .filter(|z| z.get_site(state).is_none())
                    .cloned()
                    .collect();
                if nearby_voids.is_empty() {
                    return Ok(vec![]);
                }

                let picked_void = pick_zone(
                    card.get_controller_id(),
                    &nearby_voids,
                    state,
                    "Pick a nearby void to fly to",
                )
                .await?;

                let mut effects = vec![
                    Effect::MoveCard {
                        player_id: player_id.clone(),
                        card_id: card.get_id().clone(),
                        from: card.get_zone().clone(),
                        to: ZoneQuery::Specific {
                            id: uuid::Uuid::new_v4(),
                            zone: picked_void.clone(),
                        },
                        tap: false,
                        plane: Plane::Surface,
                        through_path: None,
                    },
                    Effect::SetCardData {
                        card_id: card.get_id().clone(),
                        data: Box::new(true),
                    },
                ];

                let units_on_site = state.get_units_in_zone(card.get_zone());
                for unit in units_on_site {
                    effects.push(Effect::MoveCard {
                        player_id: player_id.clone(),
                        card_id: unit.get_id().clone(),
                        from: card.get_zone().clone(),
                        to: ZoneQuery::Specific {
                            id: uuid::Uuid::new_v4(),
                            zone: picked_void.clone(),
                        },
                        tap: false,
                        plane: unit.get_base().plane.clone(),
                        through_path: None,
                    });
                }

                Ok(effects)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloudCity {
    pub site_base: SiteBase,
    pub card_base: CardBase,
    moved_this_turn: bool,
}

impl CloudCity {
    pub const NAME: &'static str = "Cloud City";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
            moved_this_turn: false,
        }
    }
}

impl Site for CloudCity {}

#[async_trait::async_trait]
impl Card for CloudCity {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    async fn on_turn_end(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        if self.moved_this_turn {
            return Ok(vec![Effect::SetCardData {
                card_id: self.get_id().clone(),
                data: Box::new(false),
            }]);
        }

        Ok(vec![])
    }

    fn get_actions(&self, state: &State) -> anyhow::Result<Vec<Box<dyn CardAction>>> {
        if !state
            .get_player_resources(self.get_controller_id())?
            .has_resources(0, "AAA")
        {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(CloudCityAction::FlyToVoid)])
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(moved) = data.downcast_ref::<bool>() {
            self.moved_this_turn = *moved;
        }

        Ok(())
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}
