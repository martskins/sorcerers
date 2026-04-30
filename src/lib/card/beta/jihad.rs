use crate::{
    card::{
        Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, Element, Rarity, Region,
        Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct Jihad {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Jihad {
    pub const NAME: &'static str = "Jihad";
    pub const DESCRIPTION: &'static str = "Allied fire minions at affected sites have +1 power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
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
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
        }
    }
}

impl Aura for Jihad {}

#[async_trait::async_trait]
impl Card for Jihad {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let controller_id = self.get_controller_id(state);
        let affected_zones = self.get_affected_zones(state);

        let affected_minions = CardQuery::new()
            .minions()
            .in_zones(&affected_zones)
            .controlled_by(&controller_id)
            .with_affinity(Element::Fire);

        Ok(vec![ContinuousEffect::ModifyPower {
            power_diff: 1,
            affected_cards: affected_minions,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Jihad::NAME, |owner_id: PlayerId| {
    Box::new(Jihad::new(owner_id))
});
