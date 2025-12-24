use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{Direction, PlayerId, Thresholds, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MajorExplosion {
    pub card_base: CardBase,
}

impl MajorExplosion {
    pub const NAME: &'static str = "Major Explosion";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 7,
                required_thresholds: Thresholds::parse("FF"),
                plane: Plane::Surface,
                rarity: Rarity::Elite,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MajorExplosion {
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
        let zones = caster.get_zones_within_steps(state, 2);
        let prompt = "Pick a zone to center Major Explosion:";
        let zone = pick_zone(self.get_owner_id(), &zones, state, prompt).await;
        let zone_dmg: Vec<(Option<Zone>, u8)> = vec![
            (Some(zone.clone()), 7),
            (zone.zone_in_direction(&Direction::Up), 5),
            (zone.zone_in_direction(&Direction::Down), 5),
            (zone.zone_in_direction(&Direction::Left), 5),
            (zone.zone_in_direction(&Direction::Right), 5),
            (zone.zone_in_direction(&Direction::TopLeft), 3),
            (zone.zone_in_direction(&Direction::TopRight), 3),
            (zone.zone_in_direction(&Direction::BottomLeft), 3),
            (zone.zone_in_direction(&Direction::BottomRight), 3),
        ];
        let mut effects = vec![];
        for (zone, dmg) in zone_dmg {
            if let Some(z) = zone {
                let units = state.get_units_in_zone(&z);
                for unit in units {
                    effects.push(Effect::take_damage(unit.get_id(), self.get_id(), dmg));
                }
            }
        }
        effects
    }
}
