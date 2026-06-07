use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct GreyWolves {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GreyWolves {
    pub const NAME: &'static str = "Grey Wolves";
    pub const DESCRIPTION: &'static str = "Your spellbook may include any number of Grey Wolves.\r \r Has +1 power for each other Grey Wolves nearby.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for GreyWolves {
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

    async fn get_continuous_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![OngoingEffect::ModifyPowerForEach {
            power_per_card: 1,
            affected_cards: self.get_id().into(),
            matching_cards: CardQuery::new()
                .minions()
                .named(Self::NAME.to_string())
                .nearby_locations_to_card(self.get_id())
                .id_not(*self.get_id()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GreyWolves::NAME, |owner_id: PlayerId| {
    Box::new(GreyWolves::new(owner_id))
});
