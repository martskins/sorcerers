use crate::{effect::FightContext, prelude::*};

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

    fn get_valid_play_locations(
        &self,
        state: &State,
        player_id: &PlayerId,
        _caster_id: &CardId,
    ) -> anyhow::Result<Vec<Location>> {
        Ok(Location::all_in_region(Region::Underground)
            .into_iter()
            .filter(|loc| {
                let costs = state
                    .get_effective_costs(self.get_id(), Some(loc), player_id)
                    .unwrap_or_default();
                if !costs.can_afford(state, player_id).unwrap_or_default() {
                    return false;
                }

                if !loc
                    .is_valid_play_location_for(state, self.get_id(), player_id)
                    .unwrap_or_default()
                {
                    return false;
                }

                loc.get_site(state)
                    .is_some_and(|site| !site.is_water_site(state).unwrap_or_default())
            })
            .collect())
    }

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let player_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&player_id)?;
        Ok(vec![Hook {
            id: ENTER_ZONE_HOOK,
            trigger: EffectQuery::EnterZone {
                card: CardQuery::new().minions().controlled_by(&opponent_id),
                zone: ZoneQuery::from_location(self.get_location().with_region(Region::Surface)),
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
                let enemy_ids = match effect {
                    Effect::SummonCards { summoned_cards } => {
                        let mut output = vec![];
                        for sc in summoned_cards {
                            if sc.to_location.square() != self.get_location().square() {
                                continue;
                            }

                            if let Location::Square(_, region) = &sc.to_location
                                && *region != Region::Surface
                            {
                                continue;
                            }

                            output.push(&sc.card_id);
                        }

                        output
                    }
                    Effect::MoveCard { card_id, .. } => vec![card_id],
                    _ => return Ok(vec![]),
                };

                let is_burrowed = self.get_region(state) == &Region::Underground;
                if !is_burrowed {
                    return Ok(vec![]);
                }

                let mut effects = vec![Effect::SetCardRegion {
                    card_id: *self.get_id(),
                    destination: Region::Surface,
                    tap: false,
                }];

                for enemy_id in enemy_ids {
                    effects.push(Effect::Fight {
                        attacker_id: *self.get_id(),
                        defender_id: *enemy_id,
                        defending_ids: vec![],
                        damage_assignment: None,
                        context: FightContext::FightOnly,
                    })
                }

                Ok(effects)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::FootSoldier;

    #[tokio::test]
    async fn moving_enemy_above_burrowed_mummies_queues_fight_only() {
        let mut state = State::new_mock_state(vec![8]);
        let player_id = state.players[0].id;
        let opponent_id = state.players[1].id;
        let underground = Zone::Location(Location::Square(8, Region::Underground));
        let surface = Zone::Location(Location::Square(8, Region::Surface));

        let mut mummies = AwakenedMummies::new(player_id);
        let mummies_id = *mummies.get_id();
        mummies.set_zone(underground.clone());
        state.cards.insert(mummies_id, Box::new(mummies.clone()));

        let mut enemy = FootSoldier::new(opponent_id);
        let enemy_id = *enemy.get_id();
        enemy.set_zone(surface.clone());
        state.cards.insert(enemy_id, Box::new(enemy));

        let effects = mummies
            .resolve_hook(
                ENTER_ZONE_HOOK,
                &state,
                &Effect::MoveCard {
                    player_id: opponent_id,
                    card_id: enemy_id,
                    from: Location::Square(7, Region::Surface),
                    to: LocationQuery::from_zone(surface),
                    tap: true,
                    through_path: None,
                },
            )
            .await
            .unwrap();

        assert!(matches!(
            effects.as_slice(),
            [
                Effect::SetCardRegion {
                    card_id,
                    destination: Region::Surface,
                    ..
                },
                Effect::Fight {
                    attacker_id,
                    defender_id,
                    defending_ids,
                    damage_assignment: None,
                    context: FightContext::FightOnly,
                },
            ] if *card_id == mummies_id
                && *attacker_id == mummies_id
                && *defender_id == enemy_id
                && defending_ids.is_empty()
        ));
    }

    #[tokio::test]
    async fn fight_only_does_not_trigger_attack_hooks() {
        let mut state = State::new_mock_state(vec![8]);
        let player_id = state.players[0].id;
        let opponent_id = state.players[1].id;
        let zone = Zone::Location(Location::Square(8, Region::Surface));

        let mut mummies = AwakenedMummies::new(player_id);
        let mummies_id = *mummies.get_id();
        mummies.set_zone(zone.clone());
        state.cards.insert(mummies_id, Box::new(mummies));

        let mut enemy = FootSoldier::new(opponent_id);
        let enemy_id = *enemy.get_id();
        enemy.set_zone(zone.clone());
        state.cards.insert(enemy_id, Box::new(enemy));

        let fight = Effect::Fight {
            attacker_id: mummies_id,
            defender_id: enemy_id,
            defending_ids: vec![],
            damage_assignment: None,
            context: FightContext::FightOnly,
        };
        let attack_query = EffectQuery::Attack {
            attacker: CardQuery::from_id(mummies_id),
            defender: Some(CardQuery::from_id(enemy_id)),
        };

        assert!(!attack_query.matches(&fight, &state).await.unwrap());

        assert!(matches!(
            fight,
            Effect::Fight {
                context: FightContext::FightOnly,
                ..
            }
        ));
    }
}
