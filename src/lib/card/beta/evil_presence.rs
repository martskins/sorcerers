use crate::prelude::*;

const SUMMON_SPIRIT_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct EvilPresence {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl EvilPresence {
    pub const NAME: &'static str = "Evil Presence";
    pub const DESCRIPTION: &'static str = "You may summon Spirits to affected sites. When you summon a Spirit here, give it Charge, and return Evil Presence to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
        }
    }
}

impl Aura for EvilPresence {}

#[async_trait::async_trait]
impl Card for EvilPresence {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }

    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: SUMMON_SPIRIT_HOOK,
            trigger: EffectQuery::SummonCard {
                card: CardQuery::new().minions().minion_type(&MinionType::Spirit),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::Any,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let Effect::SummonCards { summoned_cards } = effect else {
                    return Ok(vec![]);
                };

                let mut effects = vec![Effect::SetCardZone {
                    card_id: *self.get_id(),
                    zone: Zone::Hand,
                }];
                for sc in summoned_cards {
                    if state
                        .get_card(&sc.card_id)
                        .get_unit_base()
                        .is_none_or(|ub| !ub.types.contains(&MinionType::Spirit))
                    {
                        continue;
                    }

                    effects.push(Effect::AddAbilityCounter {
                        card_id: sc.card_id,
                        counter: AbilityCounter {
                            id: uuid::Uuid::new_v4(),
                            ability: Ability::Charge,
                            expires_on_effect: None,
                        },
                    });
                }

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }

    async fn get_continuous_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![OngoingEffect::OverrideValidPlayZone {
            affected_zones: ZoneQuery::new().affected_zones_of_card(self.get_id()),
            affected_cards: CardQuery::new()
                .minions()
                .minion_type(&MinionType::Spirit)
                .including_not_in_play(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (EvilPresence::NAME, |owner_id: PlayerId| {
    Box::new(EvilPresence::new(owner_id))
});
