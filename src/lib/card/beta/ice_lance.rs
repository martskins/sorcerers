use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, Direction, PlayerId, Thresholds, pick_card, pick_direction},
    query::QueryCache,
    state::State,
};

#[derive(Debug, Clone)]
pub struct IceLance {
    pub card_base: CardBase,
}

impl IceLance {
    pub const NAME: &'static str = "Ice Lance";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "W"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for IceLance {
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
        let controller_id = self.get_controller_id(state);
        let prompt = "Ice Lance: Pick a direction to shoot the lance";
        let direction = pick_direction(controller_id, &CARDINAL_DIRECTIONS, state, prompt).await?;
        let caster = state.get_card(caster_id);

        Ok(vec![Effect::ShootProjectile {
            id: uuid::Uuid::new_v4(),
            player_id: controller_id.clone(),
            shooter: caster_id.clone(),
            from_zone: caster.get_zone().clone(),
            direction,
            damage: vec![3, 2, 1],
            piercing: true,
            splash_damage: None,
        }])
    }
}

// async fn shoot_projectile(state: &State, player_id: &PlayerId, from_zone: &Zone, direction: &Direction) -> Vec<Effect> {
//     let mut effects = vec![];
//     let mut next_zone = from_zone.zone_in_direction(direction, 1);
//     while let Some(zone) = next_zone {
//         let picked_unit_id = match self.affected_cards().await {
//             Some(affected_cards) => affected_cards.first().cloned(),
//             None => {
//                 let units = state
//                     .get_units_in_zone(&zone)
//                     .iter()
//                     .filter(|c| c.can_be_targetted_by(state, player_id))
//                     .map(|c| c.get_id().clone())
//                     .collect::<Vec<_>>();
//                 match units.len() {
//                     0 => None,
//                     1 => Some(units[0].clone()),
//                     _ => {
//                         let prompt = "Pick a unit to shoot";
//                         let picked_unit_id = pick_card(player_id, &units, state, prompt).await?;
//                         QueryCache::store_effect_targets(
//                             state.game_id.clone(),
//                             id.clone(),
//                             vec![picked_unit_id.clone()],
//                         )
//                         .await;
//                         Some(picked_unit_id)
//                     }
//                 }
//             }
//         };
//
//         if let Some(picked_unit_id) = picked_unit_id {
//             effects.push(Effect::take_damage(&picked_unit_id, shooter, *damage));
//             if let Some(splash_damage) = splash_damage {
//                 let splash_effects = state
//                     .get_units_in_zone(&zone)
//                     .iter()
//                     .filter(|c| c.get_id() != &picked_unit_id)
//                     .map(|c| Effect::take_damage(c.get_id(), shooter, *splash_damage))
//                     .collect::<Vec<_>>();
//                 effects.extend(splash_effects);
//             }
//
//             if !piercing {
//                 break;
//             }
//         }
//
//         next_zone = zone.zone_in_direction(direction, 1);
//     }
//
//     effects
// }

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (IceLance::NAME, |owner_id: PlayerId| Box::new(IceLance::new(owner_id)));