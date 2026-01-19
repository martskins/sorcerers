use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Direction, PlayerId, pick_zone},
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
                cost: Cost::new(7, "FF"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let zones = caster.get_zones_within_steps(state, 2);
        let prompt = "Pick a zone to center Major Explosion:";
        let zone = pick_zone(self.get_owner_id(), &zones, state, false, prompt).await?;
        let zone_dmg: Vec<(Option<Zone>, u16)> = vec![
            (Some(zone.clone()), 7),
            (zone.zone_in_direction(&Direction::Up, 1), 5),
            (zone.zone_in_direction(&Direction::Down, 1), 5),
            (zone.zone_in_direction(&Direction::Left, 1), 5),
            (zone.zone_in_direction(&Direction::Right, 1), 5),
            (zone.zone_in_direction(&Direction::TopLeft, 1), 3),
            (zone.zone_in_direction(&Direction::TopRight, 1), 3),
            (zone.zone_in_direction(&Direction::BottomLeft, 1), 3),
            (zone.zone_in_direction(&Direction::BottomRight, 1), 3),
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
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (MajorExplosion::NAME, |owner_id: PlayerId| {
    Box::new(MajorExplosion::new(owner_id))
});
