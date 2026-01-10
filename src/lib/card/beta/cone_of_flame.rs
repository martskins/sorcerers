use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction},
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
                cost: Cost::new(2, "F"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let prompt = "Cone of Flame: Pick a direction to cast the spell:";
        let dir = pick_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS, state, prompt).await?;
        let caster = state.get_card(caster_id);
        let zone = caster.get_zone();
        let zone_dmg = vec![
            (zone.zone_in_direction(&dir, 1), 5),
            (zone.zone_in_direction(&dir, 2), 3),
            (zone.zone_in_direction(&dir, 3), 1),
            (zone.zone_in_direction(&dir.rotate(1)?, 1), 3),
            (zone.zone_in_direction(&dir.rotate(1)?, 2), 1),
            (zone.zone_in_direction(&dir.rotate(7)?, 1), 3),
            (zone.zone_in_direction(&dir.rotate(7)?, 2), 1),
        ];

        let mut effects = vec![];
        for (zone, dmg) in zone_dmg {
            if let Some(zone) = zone {
                let units = state.get_units_in_zone(&zone);
                for unit in units {
                    effects.push(Effect::take_damage(unit.get_id(), self.get_id(), dmg));
                }
            }
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (ConeOfFlame::NAME, |owner_id: PlayerId| {
    Box::new(ConeOfFlame::new(owner_id))
});
