use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{Action, BaseAction, PlayerId, Thresholds, pick_action},
    state::State,
};

#[derive(Debug, Clone)]
pub struct WayfaringPilgrim {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
    corners_visited: Vec<Zone>,
}

impl WayfaringPilgrim {
    pub const NAME: &'static str = "Wayfaring Pilgrim";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                modifiers: vec![Modifier::Airborne],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Air,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    async fn on_visit_zone(&self, state: &State, to: &Zone) -> Vec<Effect> {
        if !to.is_in_realm() {
            return vec![];
        }

        let is_corner = [1, 5, 16, 20].contains(&to.get_square().unwrap());
        if !is_corner {
            return vec![];
        }

        let mut corners_visited = self.corners_visited.clone();
        if corners_visited.contains(to) {
            return vec![];
        }

        corners_visited.push(to.clone());
        let actions: Vec<Box<dyn Action>> = vec![Box::new(BaseAction::DrawSite), Box::new(BaseAction::DrawSpell)];
        let prompt = "Wayfaring Pilgrim: Draw a card";
        let picked_action = pick_action(self.get_owner_id(), &actions, state, prompt).await;
        let mut effects = picked_action
            .on_select(Some(self.get_id()), self.get_owner_id(), state)
            .await;

        effects.push(Effect::SetCardData {
            card_id: self.get_id().clone(),
            data: Box::new(corners_visited),
        });
        effects
    }
}
