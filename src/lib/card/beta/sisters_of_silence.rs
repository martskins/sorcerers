use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SistersOfSilence {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SistersOfSilence {
    pub const NAME: &'static str = "Sisters of Silence";
    pub const DESCRIPTION: &'static str = "Other nearby minions are silenced.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
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
impl Card for SistersOfSilence {
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

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::RemoveAbilities {
            removal: AbilityRemoval::SpecialAbilities,
            affected_cards: CardQuery::new()
                .minions()
                .nearby_zones_to_card(self.get_id())
                .id_not(self.get_id()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SistersOfSilence::NAME, |owner_id: PlayerId| {
        Box::new(SistersOfSilence::new(owner_id))
    });
