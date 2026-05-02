use crate::{
    card::{Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone},
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct SummoningSphere {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl SummoningSphere {
    pub const NAME: &'static str = "Summoning Sphere";
    pub const DESCRIPTION: &'static str = "You may summon minions to affected sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "A"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Aura for SummoningSphere {}

#[async_trait::async_trait]
impl Card for SummoningSphere {
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
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        let controller_id = self.get_controller_id(state);
        Ok(vec![ContinuousEffect::OverrideValidPlayZone {
            affected_zones: self.get_affected_zones(state),
            affected_cards: CardQuery::new()
                .minions()
                .controlled_by(&controller_id)
                .including_not_in_play(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SummoningSphere::NAME, |owner_id: PlayerId| {
        Box::new(SummoningSphere::new(owner_id))
    });
