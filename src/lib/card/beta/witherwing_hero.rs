use crate::prelude::*;

const WEAKER_MINION_ATTACKED_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct WitherwingHero {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl WitherwingHero {
    pub const NAME: &'static str = "Witherwing Hero";
    pub const DESCRIPTION: &'static str = "Airborne
        Whenever a weaker allied minion here is attacked, you may return it to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WitherwingHero {
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
        let player_id = self.get_controller_id(state);
        let power = self.get_power(state)?.unwrap_or_default();
        Ok(vec![Hook {
            id: WEAKER_MINION_ATTACKED_HOOK,
            trigger: EffectQuery::Attack {
                attacker: CardQuery::new().units(),
                defender: Some(
                    CardQuery::new()
                        .minions()
                        .controlled_by(&player_id)
                        .in_zone_of_card(self.get_id())
                        .power_gte(power),
                ),
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
            WEAKER_MINION_ATTACKED_HOOK => {
                let Effect::DeclareAttack { target_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let should_return = yes_or_no_source(
                    &self.get_controller_id(state),
                    state,
                    "Return the attacked ally to its owner's hand?",
                    Some(*self.get_id()),
                )
                .await?;
                if !should_return {
                    return Ok(vec![]);
                }
                Ok(vec![Effect::SetCardZone {
                    card_id: *target_id,
                    zone: Zone::Hand,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WitherwingHero::NAME, |owner_id: PlayerId| {
        Box::new(WitherwingHero::new(owner_id))
    });
