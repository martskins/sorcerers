use crate::prelude::*;

const MINION_DEATH_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct SquirmingMass {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SquirmingMass {
    pub const NAME: &'static str = "Squirming Mass";
    pub const DESCRIPTION: &'static str =
        "Whenever another nearby minion dies, Squirming Mass permanently gains its power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 0,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "EE"),
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
impl Card for SquirmingMass {
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
            id: MINION_DEATH_HOOK,
            trigger: EffectQuery::BuryCard {
                card: Box::new(CardQuery::new()
                    .minions()
                    .nearby_locations_to_card(self.get_id())),
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
            MINION_DEATH_HOOK => {
                let Effect::BuryCard { card_id } = effect else {
                    return Ok(vec![]);
                };

                let buried_card = state.get_card(card_id);
                let power = buried_card
                    .get_unit_base()
                    .map(|ub| ub.power as i16)
                    .unwrap_or(0);
                if power <= 0 {
                    return Ok(vec![]);
                }
                Ok(vec![Effect::AddCounter {
                    card_id: *self.get_id(),
                    counter: Counter::new(power, power, None),
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SquirmingMass::NAME, |owner_id: PlayerId| {
        Box::new(SquirmingMass::new(owner_id))
    });
