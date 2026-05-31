use crate::prelude::*;

#[derive(Debug, Clone)]
struct DiscardToGainControlAbility;

#[async_trait::async_trait]
impl ActivatedAbility for DiscardToGainControlAbility {
    fn get_name(&self) -> String {
        "Discard a card to gain control of Seasoned Sellsword".to_string()
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::SetController {
            card_id: *card_id,
            player_id: *player_id,
        }])
    }

    fn get_cost(&self, _card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::discard(
            CardQuery::new().in_zone(&Zone::Hand),
        )))
    }
}

#[derive(Debug, Clone)]
pub struct SeasonedSellsword {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SeasonedSellsword {
    pub const NAME: &'static str = "Seasoned Sellsword";
    pub const DESCRIPTION: &'static str =
        "Avatars nearby have \"Discard a card → Gain control of Seasoned Sellsword.\"";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "FF"),
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
impl Card for SeasonedSellsword {
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
        Ok(vec![ContinuousEffect::GrantActivatedAbility {
            ability: Box::new(DiscardToGainControlAbility),
            affected_cards: CardQuery::new().avatars().in_play(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SeasonedSellsword::NAME, |owner_id: PlayerId| {
        Box::new(SeasonedSellsword::new(owner_id))
    });
