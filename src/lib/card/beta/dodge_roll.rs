use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DodgeRoll {
    card_base: CardBase,
}

const EVADE_ATTACK_HOOK: HookId = 1;

impl DodgeRoll {
    pub const NAME: &'static str = "Dodge Roll";
    pub const DESCRIPTION: &'static str = "May be cast when an ally is attacked.\r \r An attacked ally may move to another adjacent location to evade the attack.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(0, "WW"),
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
impl Card for DodgeRoll {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let controller_id = self.get_controller_id(state);
        let dodge_rolls_in_hand = CardQuery::new()
            .named(DodgeRoll::NAME.to_string())
            .controlled_by(&controller_id)
            .including_not_in_play()
            .in_zone(&Zone::Hand)
            .all(state);
        if dodge_rolls_in_hand.first() != Some(self.get_id()) {
            return Ok(vec![]);
        }

        Ok(vec![Hook {
            id: EVADE_ATTACK_HOOK,
            trigger: EffectQuery::Attack {
                attacker: CardQuery::new().units(),
                defender: Some(CardQuery::new().units().controlled_by(&controller_id)),
            },
            timing: HookTiming::Replace,
            source_zones: HookSourceZones::Zone(Zone::Hand),
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            EVADE_ATTACK_HOOK => {
                let Effect::DeclareAttack {
                    target_id,
                    attacker_id,
                } = effect
                else {
                    return Ok(vec![]);
                };

                let defender = state.get_card(target_id);
                let defender_controller = defender.get_controller_id(state);
                let prompt = format!(
                    "Use Dodge Roll to evade the attack on {}?",
                    defender.get_name()
                );
                let use_dodge_roll =
                    yes_or_no_source(defender_controller, state, prompt, Some(*self.get_id()))
                        .await?;
                if !use_dodge_roll {
                    return Ok(vec![]);
                }

                let avatar_id = state.get_player_avatar_id(&defender_controller)?;
                let avatar = state.get_card(&avatar_id);
                let adjacent_locations = defender.get_location().get_adjacent_locations(state);
                let prompt = "Dodge Roll: Pick an adjacent location to move to";
                let picked_site = pick_location_source(
                    defender_controller,
                    &adjacent_locations,
                    state,
                    true,
                    prompt,
                    Some(*self.get_id()),
                )
                .await?;

                let attacker = state.get_card(attacker_id);
                let attacker_controller = attacker.get_controller_id(state);
                Ok(vec![
                    Effect::SetCardZone {
                        card_id: *target_id,
                        zone: picked_site.into(),
                    },
                    Effect::MoveCard {
                        player_id: attacker_controller,
                        card_id: *attacker_id,
                        from: attacker
                            .get_zone()
                            .clone()
                            .location().cloned()
                            .expect("Dodge Roll attacker must be in a location"),
                        to: LocationQuery::from_location(defender.get_location().clone()),
                        tap: true,
                        through_path: None,
                    },
                    Effect::PlayMagic {
                        player_id: defender_controller,
                        card_id: *self.get_id(),
                        caster_id: avatar_id,
                        from: avatar
                            .get_zone()
                            .clone()
                            .location().cloned()
                            .expect("Dodge Roll caster must be in a location"),
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl Magic for DodgeRoll {
    async fn resolve_magic(
        &self,
        _state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (DodgeRoll::NAME, |owner_id: PlayerId| {
    Box::new(DodgeRoll::new(owner_id))
});
