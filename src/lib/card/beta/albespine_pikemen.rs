use crate::prelude::*;

const GAIN_FIRST_STRIKE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct AlbespinePikemen {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl AlbespinePikemen {
    pub const NAME: &'static str = "Albespine Pikemen";
    pub const DESCRIPTION: &'static str = "Strikes first while attacking.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "EE"),
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
impl Card for AlbespinePikemen {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: GAIN_FIRST_STRIKE_HOOK,
            trigger: EffectQuery::Attack {
                attacker: self.get_id().into(),
                defender: None,
            },
            timing: HookTiming::Before,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GAIN_FIRST_STRIKE_HOOK => {
                let Effect::DeclareAttack { target_id, .. } = effect else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::AddTemporaryEffect {
                    effect: Box::new(TemporaryEffect::GrantAbility {
                        ability: Ability::FirstStrike,
                        affected_cards: self.get_id().into(),
                        expires_on_effect: Box::new(EffectQuery::Fight {
                            attacker: self.get_id().into(),
                            defender: Some(target_id.into()),
                        }),
                    }),
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AlbespinePikemen::NAME, |owner_id: PlayerId| {
        Box::new(AlbespinePikemen::new(owner_id))
    });
