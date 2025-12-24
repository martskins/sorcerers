use crate::{
    card::{AuraBase, Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Wildfire {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
    sites_visited: Vec<Zone>,
}

impl Wildfire {
    pub const NAME: &'static str = "Wildfire";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 4,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
            },
            aura_base: AuraBase {},
            sites_visited: vec![],
        }
    }
}

#[async_trait::async_trait]
impl Card for Wildfire {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(sites_visited) = data.downcast_ref::<Vec<Zone>>() {
            self.sites_visited = sites_visited.clone();
        }

        Ok(())
    }

    async fn on_visit_zone(&self, _state: &State, to: &Zone) -> Vec<Effect> {
        let mut sites_visited = self.sites_visited.clone();
        if sites_visited.is_empty() {
            sites_visited.push(self.get_zone().clone());
        }

        sites_visited.push(to.clone());
        vec![Effect::SetCardData {
            card_id: self.get_id().clone(),
            data: Box::new(sites_visited),
        }]
    }

    async fn on_turn_end(&self, state: &State) -> Vec<Effect> {
        let zones = self
            .get_zone()
            .get_adjacent()
            .iter()
            .filter(|z| !self.sites_visited.contains(z))
            .cloned()
            .collect::<Vec<Zone>>();
        if zones.is_empty() {
            return vec![Effect::BuryCard {
                card_id: self.get_id().clone(),
                from: self.get_zone().clone(),
            }];
        }

        let mut effects: Vec<Effect> = state
            .get_units_in_zone(self.get_zone())
            .iter()
            .map(|c| Effect::take_damage(c.get_id(), self.get_id(), 3))
            .collect();
        let prompt = "Wildfire: Pick a zone to move to:";
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, prompt).await;
        effects.push(Effect::MoveCard {
            card_id: self.get_id().clone(),
            from: self.get_zone().clone(),
            to: picked_zone.clone(),
            tap: false,
        });

        effects
    }
}
