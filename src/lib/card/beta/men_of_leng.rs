use crate::prelude::*;

const ATTACK_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct MenOfLeng {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MenOfLeng {
    pub const NAME: &'static str = "Men of Leng";
    pub const DESCRIPTION: &'static str =
        "Whenever Men of Leng strike an Avatar, that Avatar discards a random card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MenOfLeng {
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
            id: ATTACK_HOOK,
            trigger: EffectQuery::Attack {
                attacker: self.get_id().into(),
                defender: Some(Box::new(CardQuery::new().avatars())),
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
            ATTACK_HOOK => {
                let Effect::DeclareAttack { target_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let avatar = state.get_card(target_id);
                let avatar_controller = avatar.get_controller_id(state);

                let random_card = CardQuery::new()
                    .in_zone(&Zone::Hand)
                    .controlled_by(&avatar_controller)
                    .randomised()
                    .count(1)
                    .pick(&avatar_controller, state)
                    .await?;

                if let Some(card_id) = random_card {
                    Ok(vec![Effect::DiscardCard {
                        player_id: avatar_controller,
                        card_id,
                    }])
                } else {
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MenOfLeng::NAME, |owner_id: PlayerId| {
    Box::new(MenOfLeng::new(owner_id))
});
