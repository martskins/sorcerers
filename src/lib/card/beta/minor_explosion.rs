use crate::{
    card::{Card, CardBase, Edition, Plane, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MinorExplosion {
    pub card_base: CardBase,
}

impl MinorExplosion {
    pub const NAME: &'static str = "Minor Explosion";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("FF"),
                plane: Plane::Surface,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MinorExplosion {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
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

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> Vec<Effect> {
        let caster = state.get_card(caster_id).unwrap();
        let valid_zones = caster.get_zones_within_steps(state, 2);
        let zone = pick_zone(self.get_owner_id(), &valid_zones, state).await;
        let units = state.get_units_in_zone(&zone);
        units
            .iter()
            .map(|c| Effect::take_damage(c.get_id(), self.get_id(), 3))
            .collect()
    }
}
