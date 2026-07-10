use crate::prelude::*;

const LIFE_LOSS_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct MordricDruids {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MordricDruids {
    pub const NAME: &'static str = "Mordric Druids";
    pub const DESCRIPTION: &'static str = "Spellcaster\r \r Whenever you lose life due to an undefended attack nearby, the attacker's controller also loses that much life.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Spellcaster(None)],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MordricDruids {
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
        let controller_id = self.get_controller_id(state);
        Ok(vec![Hook {
            id: LIFE_LOSS_HOOK,
            trigger: EffectQuery::DamageDealt {
                source: None,
                target: Some(Box::new(
                    CardQuery::new()
                        .card_types(vec![CardType::Avatar, CardType::Site])
                        .controlled_by(&controller_id)
                        .nearby_to_card(self.get_id()),
                )),
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
            LIFE_LOSS_HOOK => {
                let Effect::TakeDamage { from, damage, .. } = effect else {
                    return Ok(vec![]);
                };
                if !damage.is_attack || damage.amount == 0 {
                    return Ok(vec![]);
                }

                let attacker_controller = state.get_card(from).get_controller_id(state);
                Ok(vec![Effect::AdjustAvatarLife {
                    player_id: attacker_controller,
                    amount: -i16::try_from(damage.amount).unwrap_or(0),
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MordricDruids::NAME, |owner_id: PlayerId| {
        Box::new(MordricDruids::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::FootSoldier;

    fn setup() -> (State, MordricDruids, CardId, PlayerId) {
        let mut state = State::new_mock_state(vec![1, 2]);
        let controller_id = state.players[0].id;
        let attacker_controller_id = state.players[1].id;

        let mut druids = MordricDruids::new(controller_id);
        druids.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
        state.add_card(Box::new(druids.clone()));

        let mut attacker = FootSoldier::new(attacker_controller_id);
        let attacker_id = *attacker.get_id();
        attacker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
        state.add_card(Box::new(attacker));

        (state, druids, attacker_id, attacker_controller_id)
    }

    #[tokio::test]
    async fn reflects_undefended_attack_damage_to_nearby_allied_site() {
        let (state, druids, attacker_id, attacker_controller_id) = setup();
        let site_id = CardQuery::new()
            .sites()
            .in_location(Location::Square(2, Region::Surface))
            .all(&state)[0];
        let effect = Effect::TakeDamage {
            card_id: site_id,
            from: attacker_id,
            damage: Damage::attack(3),
        };

        let hook = druids.hooks(&state).unwrap().pop().unwrap();
        assert!(hook.trigger.matches(&effect, &state).await.unwrap());
        assert!(matches!(
            druids.resolve_hook(LIFE_LOSS_HOOK, &state, &effect).await.unwrap().as_slice(),
            [Effect::AdjustAvatarLife { player_id, amount }]
                if *player_id == attacker_controller_id && *amount == -3
        ));
    }

    #[tokio::test]
    async fn reflects_undefended_attack_damage_to_nearby_allied_avatar() {
        let (mut state, druids, attacker_id, attacker_controller_id) = setup();
        let avatar_id = state.get_player_avatar_id(&state.players[0].id).unwrap();
        state
            .get_card_mut(&avatar_id)
            .set_zone(Zone::Location(Location::Square(2, Region::Surface)));
        let effect = Effect::TakeDamage {
            card_id: avatar_id,
            from: attacker_id,
            damage: Damage::attack(2),
        };

        let hook = druids.hooks(&state).unwrap().pop().unwrap();
        assert!(hook.trigger.matches(&effect, &state).await.unwrap());
        assert!(matches!(
            druids.resolve_hook(LIFE_LOSS_HOOK, &state, &effect).await.unwrap().as_slice(),
            [Effect::AdjustAvatarLife { player_id, amount }]
                if *player_id == attacker_controller_id && *amount == -2
        ));
    }

    #[tokio::test]
    async fn ignores_non_attack_damage() {
        let (state, druids, attacker_id, _) = setup();
        let site_id = CardQuery::new()
            .sites()
            .in_location(Location::Square(2, Region::Surface))
            .all(&state)[0];
        let effect = Effect::TakeDamage {
            card_id: site_id,
            from: attacker_id,
            damage: Damage::basic(3),
        };

        assert!(
            druids
                .resolve_hook(LIFE_LOSS_HOOK, &state, &effect)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
