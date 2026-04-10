use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_card, pick_direction, yes_or_no},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct GrappleShot {
    pub card_base: CardBase,
}

impl GrappleShot {
    pub const NAME: &'static str = "Grapple Shot";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GrappleShot {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let ally_ids = CardMatcher::new()
            .with_card_types(vec![CardType::Minion, CardType::Avatar])
            .with_controller_id(&controller_id)
            .resolve_ids(state);
        let ally_id = pick_card(
            &controller_id,
            &ally_ids,
            state,
            "Grapple Shot: Pick an ally to shoot the projectile",
        )
        .await?;

        let ally_card = state.get_card(&ally_id);
        let ally_zone = ally_card.get_zone();
        let direction = pick_direction(
            &controller_id,
            &CARDINAL_DIRECTIONS,
            state,
            "Grapple Shot: Pick a direction",
        )
        .await?;
        let mut cur_zone = ally_zone.clone();
        let mut hit_unit_id = None;
        loop {
            match cur_zone.zone_in_direction(&direction, 1) {
                Some(Zone::Realm(next_sq)) if next_sq >= 1 && next_sq <= 20 => {
                    cur_zone = Zone::Realm(next_sq);
                    let units = cur_zone.get_units(state, None);
                    for unit in units {
                        if unit.is_unit() {
                            hit_unit_id = Some(unit.get_id());
                            break;
                        }
                    }
                    if hit_unit_id.is_some() {
                        break;
                    }
                }
                _ => break,
            }
        }

        if let Some(target_id) = hit_unit_id {
            let mut effects = vec![Effect::MoveCard {
                player_id: controller_id,
                card_id: ally_id,
                from: ally_zone.clone(),
                to: cur_zone.clone().into(),
                tap: false,
                region: ally_card.get_base().region.clone(),
                through_path: None,
            }];
            // 5. Ask if you want to strike the hit unit
            let strike = yes_or_no(&controller_id, state, "Strike the hit unit?")
                .await
                .unwrap_or(false);
            if strike {
                effects.push(Effect::Attack {
                    attacker_id: ally_id,
                    defender_id: target_id.clone(),
                });
            }
            Ok(effects)
        } else {
            Ok(vec![])
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (GrappleShot::NAME, |owner_id: PlayerId| {
    Box::new(GrappleShot::new(owner_id))
});
