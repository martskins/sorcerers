use crate::{
    card::{Card, CardBase, Edition, MinionType, Plane, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Incinerate {
    pub card_base: CardBase,
}

impl Incinerate {
    pub const NAME: &'static str = "Incinerate";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Incinerate {
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
        let mut zones: Vec<Zone> = state
            .cards
            .iter()
            .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| c.is_unit())
            .filter(|c| c.get_unit_base().unwrap().types.contains(&MinionType::Dragon))
            .flat_map(|c| c.get_zone().get_nearby())
            .collect();
        zones.push(caster.get_zone().clone());

        let picked_zone = pick_zone(self.get_owner_id(), &zones, state.get_sender(), state.get_receiver()).await;
        let units = state.get_units_in_zone(&picked_zone);
        let mut effects = vec![];
        for unit in units {
            if unit.get_id() == self.get_id() {
                continue;
            }

            effects.push(Effect::take_damage(unit.get_id(), self.get_id(), 4));
        }
        effects
    }
}
