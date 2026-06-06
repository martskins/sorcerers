use crate::prelude::*;

const ENTER_ZONE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct AwakenedMummies {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl AwakenedMummies {
    pub const NAME: &'static str = "Awakened Mummies";
    pub const DESCRIPTION: &'static str = "Summon Awakened Mummies burrowed safely. When an enemy unit moves onto the ground above them, they unburrow and fight.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Burrowing],
                types: vec![MinionType::Undead],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "F"),
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
impl Card for AwakenedMummies {
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

    fn get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
        _caster_id: &CardId,
    ) -> anyhow::Result<Vec<Zone>> {
        Ok(Zone::all_in_region(Region::Underground)
            .into_iter()
            .filter(|zone| {
                let costs = state
                    .get_effective_costs(self.get_id(), Some(zone), player_id)
                    .unwrap_or_default();
                if !costs.can_afford(state, player_id).unwrap_or_default() {
                    return false;
                }

                if !zone
                    .is_valid_play_zone_for(state, self.get_id(), player_id)
                    .unwrap_or_default()
                {
                    return false;
                }

                zone.get_site(state)
                    .is_some_and(|site| !site.is_water_site(state).unwrap_or_default())
            })
            .collect())
    }

    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let player_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&player_id)?;
        Ok(vec![Hook {
            id: ENTER_ZONE_HOOK,
            trigger: EffectQuery::EnterZone {
                card: CardQuery::new().minions().controlled_by(&opponent_id),
                zone: ZoneQuery::from_zone(self.get_zone().with_region(Region::Surface)),
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
            ENTER_ZONE_HOOK => {
                let enemy_id = match effect {
                    Effect::SummonCards { cards } => {
                        let mut output = None;
                        for (_, card_id, zone, location) in cards {
                            if zone != self.get_zone() {
                                continue;
                            }

                            if let Location::Square(_, region) = location
                                && region != &Region::Surface
                            {
                                continue;
                            }

                            output = Some(*card_id);
                        }

                        match output {
                            Some(card_id) => card_id,
                            None => return Ok(vec![]),
                        }
                    }
                    Effect::MoveCard { card_id, .. } => *card_id,
                    _ => return Ok(vec![]),
                };
                let is_burrowed = self.get_region(state) == &Region::Underground;
                if !is_burrowed {
                    return Ok(vec![]);
                }

                Ok(vec![
                    Effect::SetCardRegion {
                        card_id: *self.get_id(),
                        destination: Region::Surface,
                        tap: false,
                    },
                    // TODO: We need to separate attack into declare attackers, declare defenders
                    // and fight. After that, this should be a fight, so that no defenders can be
                    // declared here.
                    Effect::Attack {
                        attacker_id: *self.get_id(),
                        defender_id: enemy_id,
                        defending_ids: vec![],
                        damage_assignment: None,
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AwakenedMummies::NAME, |owner_id: PlayerId| {
        Box::new(AwakenedMummies::new(owner_id))
    });
