use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

/// **Root Spider** — Exceptional Minion (3 cost, 2/1)
///
/// Burrowing. While Root Spider is burrowed, minions directly above it are disabled.
#[derive(Debug, Clone)]
pub struct RootSpider {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RootSpider {
    pub const NAME: &'static str = "Root Spider";
    pub const DESCRIPTION: &'static str =
        "Burrowing\n\nWhile Root Spider is burrowed, minions directly above it are disabled.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 1,
                abilities: vec![Ability::Burrowing],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
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
impl Card for RootSpider {
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
    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        if self.get_region(state) != &Region::Underground {
            return Ok(vec![]);
        }
        let surface_minions = CardQuery::new()
            .units()
            .in_zone(self.get_zone())
            .in_region(&Region::Surface);
        Ok(vec![ContinuousEffect::GrantAbility {
            ability: Ability::Disabled,
            affected_cards: surface_minions,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RootSpider::NAME, |owner_id: PlayerId| {
    Box::new(RootSpider::new(owner_id))
});
