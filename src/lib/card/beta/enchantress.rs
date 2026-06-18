use crate::prelude::*;

const ANIMATE_AURA_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct Enchantress {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Enchantress {
    pub const NAME: &'static str = "Enchantress";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r Whenever you cast a spell, you may animate target aura until your next turn. It's an aura minion with power equal to its cost.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Enchantress {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_avatar(&self) -> Option<&dyn Avatar> {
        Some(self)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: ANIMATE_AURA_HOOK,
            trigger: EffectQuery::PlayCard {
                card: CardQuery::new().including_not_in_play(),
                spellcaster: None,
            },
            timing: HookTiming::Before,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            ANIMATE_AURA_HOOK => {
                let (spell_id, caster_id) = match effect {
                    Effect::PlayMagic {
                        card_id, caster_id, ..
                    } => (*card_id, *caster_id),
                    Effect::PlayCard {
                        card_id,
                        spellcaster,
                        ..
                    } => {
                        if state.get_card(card_id).is_site() {
                            return Ok(vec![]);
                        }
                        (*card_id, *spellcaster)
                    }
                    _ => return Ok(vec![]),
                };

                let controller_id = self.get_controller_id(state);
                let caster = state.get_card(&caster_id);
                if caster.get_controller_id(state) != controller_id {
                    return Ok(vec![]);
                }

                let aura_query = CardQuery::new()
                    .auras()
                    .in_play()
                    .can_be_targeted_by_player(&controller_id)
                    .with_source_card(*self.get_id())
                    .with_prompt("Pick an aura to animate")
                    .id_not(spell_id);
                if aura_query.all(state).is_empty() {
                    return Ok(vec![]);
                }

                let want = yes_or_no(
                    &controller_id,
                    state,
                    "Animate target aura until your next turn?",
                    *self.get_id(),
                )
                .await?;
                if !want {
                    return Ok(vec![]);
                }

                let Some(aura_id) = aura_query.pick(&controller_id, state).await? else {
                    return Ok(vec![]);
                };

                let aura = state.get_card(&aura_id);
                let power = aura
                    .get_costs(state)?
                    .printed_mana_value()
                    .unwrap_or_default() as u16;

                Ok(vec![Effect::Animate {
                    card_id: aura_id,
                    unit_base: UnitBase {
                        power,
                        toughness: power,
                        tapped: false,
                        ..Default::default()
                    },
                    expires_on_effect: EffectQuery::TurnStart {
                        player_id: Some(controller_id),
                    },
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

impl Avatar for Enchantress {}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Enchantress::NAME, |owner_id: PlayerId| {
    Box::new(Enchantress::new(owner_id))
});
