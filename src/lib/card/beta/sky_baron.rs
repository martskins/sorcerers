use crate::prelude::*;

/// **Sky Baron** — Elite Minion (6 cost, 6/6)
///
/// Airborne. All other minions lose Airborne.
#[derive(Debug, Clone)]
pub struct SkyBaron {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SkyBaron {
    pub const NAME: &'static str = "Sky Baron";
    pub const DESCRIPTION: &'static str = "Airborne\n\nAll other minions lose Airborne.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Spirit],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "AA"),
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
impl Card for SkyBaron {
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
    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        Ok(vec![OngoingEffect::RemoveAbilities {
            removal: AbilityRemoval::exact(Ability::Airborne),
            affected_cards: Box::new(CardQuery::new().minions().id_not(*self.get_id())),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SkyBaron::NAME, |owner_id: PlayerId| {
    Box::new(SkyBaron::new(owner_id))
});
