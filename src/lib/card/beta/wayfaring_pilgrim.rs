use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{BaseAction, PlayerId, pick_option},
    state::State,
};

#[derive(Debug, Clone)]
pub struct WayfaringPilgrim {
    unit_base: UnitBase,
    card_base: CardBase,
    corners_visited: Vec<Zone>,
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
                region: Region::Surface,
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

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(corners_visited) = data.downcast_ref::<Vec<Zone>>() {
            self.corners_visited = corners_visited.clone();
        }

        Ok(())
    }

    async fn on_visit_zone(
        &self,
        state: &State,
        _from: &Zone,
        to: &Zone,
    ) -> anyhow::Result<Vec<Effect>> {
        if !to.is_in_play() {
            return Ok(vec![]);
        }

        let is_corner = [1, 5, 16, 20].contains(&to.get_square().unwrap());
        if !is_corner {
            return Ok(vec![]);
        }

        let mut corners_visited = self.corners_visited.clone();
        if corners_visited.contains(to) {
            return Ok(vec![]);
        }

        corners_visited.push(to.clone());
        let options: Vec<BaseAction> = vec![BaseAction::DrawSite, BaseAction::DrawSpell];
        let option_labels: Vec<String> = options.iter().map(|a| a.get_name().to_string()).collect();
        let prompt = "Wayfaring Pilgrim: Draw a card";
        let picked_option_idx = pick_option(
            self.get_controller_id(state),
            &option_labels,
            state,
            prompt,
            false,
        )
        .await?;
        let mut effects = options[picked_option_idx]
            .on_select(&self.get_controller_id(state), state)
            .await?;

        effects.push(Effect::SetCardData {
            card_id: *self.get_id(),
            data: Box::new(corners_visited),
        });
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WayfaringPilgrim::NAME, |owner_id: PlayerId| {
        Box::new(WayfaringPilgrim::new(owner_id))
    });
