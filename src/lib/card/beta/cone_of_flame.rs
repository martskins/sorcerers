use crate::{
    card::{Card, CardBase, Edition, Plane, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, Direction, PlayerId, Thresholds, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ConeOfFlame {
    pub card_base: CardBase,
}

impl ConeOfFlame {
    pub const NAME: &'static str = "Cone of Flame";

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
impl Card for ConeOfFlame {
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
        let dir = pick_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS, state).await;
        let caster = state.get_card(caster_id).unwrap();
        let zone = caster.get_zone();
        let zone_dmg = vec![
            (zone.zone_in_direction_and_steps(&dir, 1), 5),
            (zone.zone_in_direction_and_steps(&dir, 2), 3),
            (zone.zone_in_direction_and_steps(&dir, 3), 1),
            (zone.zone_in_direction_and_steps(&dir.rotate(1), 1), 3),
            (zone.zone_in_direction_and_steps(&dir.rotate(1), 2), 1),
            (zone.zone_in_direction_and_steps(&dir.rotate(7), 1), 3),
            (zone.zone_in_direction_and_steps(&dir.rotate(7), 2), 1),
        ];

        let mut effects = vec![];
        for (zone, dmg) in zone_dmg {
            if zone.is_none() {
                continue;
            }

            let zone = zone.unwrap();
            let units = state.get_units_in_zone(&zone);
            for unit in units {
                effects.push(Effect::take_damage(unit.get_id(), self.get_id(), dmg));
            }
        }
        effects
    }
}
