use crate::{effect::FightContext, prelude::*};

const ENTER_BODY_OF_WATER_HOOK: HookId = 1;

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

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let Some(body_of_water) = state.get_body_of_water_at(self.get_location()) else {
            return Ok(vec![]);
        };

        Ok(vec![Hook {
            id: ENTER_BODY_OF_WATER_HOOK,
            trigger: EffectQuery::EnterLocation {
                card: Box::new(CardQuery::new().units().id_not(*self.get_id())),
                location: Box::new(LocationQuery::from_locations(body_of_water)),
                from: None,
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ENTER_BODY_OF_WATER_HOOK => {
                let mut card_ids = vec![];
                match effect {
                    Effect::SummonCards { summoned_cards } => {
                        for sc in summoned_cards {
                            if sc.to_location.square() == self.get_location().square() {
                                card_ids.push(&sc.card_id);
                            }
                        }
                    }
                    Effect::MoveCard { card_id, .. } => {
                        card_ids.push(card_id);
                    }
                    _ => return Ok(vec![]),
                }

                let mut effects = vec![];
                for card_id in card_ids {
                    let shark_location = self.get_location().clone();
                    let target_location = state.get_card(card_id).get_location().clone();
                    effects.push(Effect::Fight {
                        attacker_id: *self.get_id(),
                        defender_id: *card_id,
                        defending_ids: vec![],
                        damage_assignment: None,
                        context: FightContext::FightOnly,
                    });

                    if shark_location != target_location {
                        // TODO: This is likely to move the shark from the cmeetery back to the
                        // realm.
                        effects.push(Effect::MoveCard {
                            player_id: self.get_controller_id(state),
                            card_id: *self.get_id(),
                            from: shark_location.clone(),
                            to: LocationQuery::from_location(
                                target_location.with_region(Region::Underwater),
                            ),
                            tap: false,
                            through_path: None,
                        });
                    }
                }

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GiantShark::NAME, |owner_id: PlayerId| {
    Box::new(GiantShark::new(owner_id))
});
