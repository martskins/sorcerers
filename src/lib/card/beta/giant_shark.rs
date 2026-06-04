use crate::{prelude::*, query::entered_sites};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct GiantShark {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GiantShark {
    pub const NAME: &'static str = "Giant Shark";
    pub const DESCRIPTION: &'static str = "Submerge, Waterbound\r \r Whenever another unit enters or moves between sites in this body of water, Giant Shark moves to that unit to fight it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Submerge, Ability::Waterbound],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GiantShark {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let shark_id = *self.get_id();
        let Some(body_of_water) = state.get_body_of_water_at(self.get_zone()) else {
            return Ok(vec![]);
        };

        Ok(vec![OngoingEffect::TriggeredEffect {
            trigger_on_effect: EffectQuery::EnterZone {
                card: CardQuery::new().units(),
                zone: ZoneQuery::from_options(body_of_water.clone(), None),
                from: None,
            },
            on_effect: Arc::new(move |state: &State, card_id: &CardId, effect: &Effect| {
                let body_of_water = body_of_water.clone();
                Box::pin(async move {
                    if card_id == &shark_id {
                        return Ok(vec![]);
                    }

                    let entered_this_body = entered_sites(effect, state)
                        .await?
                        .into_iter()
                        .filter(|(entered_card_id, _)| entered_card_id == card_id)
                        .any(|(_, site_zone)| body_of_water.contains(&site_zone));

                    if !entered_this_body {
                        return Ok(vec![]);
                    }

                    let shark = state.get_card(&shark_id);
                    let shark_zone = shark.get_zone().clone();
                    let target_zone = state.get_card(card_id).get_zone().clone();
                    let mut effects = vec![Effect::Attack {
                        attacker_id: shark_id,
                        defender_id: *card_id,
                        defending_ids: vec![],
                        damage_assignment: None,
                    }];

                    if shark_zone != target_zone {
                        effects.push(Effect::MoveCard {
                            player_id: shark.get_controller_id(state),
                            card_id: shark_id,
                            from: (shark_zone)
                                .into_location()
                                .expect("MoveCard source must be a location"),
                            to: LocationQuery::from_zone(
                                (target_zone).with_region(Region::Underwater),
                            ),
                            tap: false,
                            through_path: None,
                        });
                    }

                    Ok(effects)
                })
            }),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GiantShark::NAME, |owner_id: PlayerId| {
    Box::new(GiantShark::new(owner_id))
});
