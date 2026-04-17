use crate::{
    card::{Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone},
    game::{Element, PlayerId},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct Crusade {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Crusade {
    pub const NAME: &'static str = "Crusade";
    pub const DESCRIPTION: &'static str = "You may summon earth minions to affected sites. Allied earth minions occupying affected sites have +1 power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "EE"),
                rarity: Rarity::Unique,
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

impl Aura for Crusade {}

#[async_trait::async_trait]
impl Card for Crusade {
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
        let affected_zones = self.get_affected_zones(state);
        let controller_id = self.get_controller_id(state);
        Ok(vec![
            ContinuousEffect::OverrideValidPlayZone {
                affected_zones: self.get_affected_zones(state),
                affected_cards: CardQuery::new()
                    .minions()
                    .with_affinity(Element::Earth)
                    .including_not_in_play(),
            },
            ContinuousEffect::ModifyPower {
                power_diff: 1,
                affected_cards: CardQuery::new()
                    .in_zones(&affected_zones)
                    .controlled_by(&controller_id)
                    .minions()
                    .with_affinity_in(vec![Element::Earth]),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Crusade::NAME, |owner_id: PlayerId| {
    Box::new(Crusade::new(owner_id))
});
