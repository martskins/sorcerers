use crate::{
    card::{Aura, AuraBase, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    game::{Element, PlayerId},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct Crusade {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
}

impl Crusade {
    pub const NAME: &'static str = "Crusade";
    pub const DESCRIPTION: &'static str = "You may summon earth minions to affected sites. Allied earth minions occupying affected sites have +1 power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "EE"),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {},
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

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    // TODO: Missing effect to allow the player to summon earth minions on affected sites,
    // regardless of who controls those sites.
    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let affected_zones = self.get_affected_zones(state);
        let controller_id = self.get_controller_id(state);
        Ok(vec![ContinuousEffect::ModifyPower {
            power_diff: 1,
            affected_cards: CardQuery::new()
                .in_zones(&affected_zones)
                .controlled_by(&controller_id)
                .minions()
                .with_affinity_in(vec![Element::Earth]),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Crusade::NAME, |owner_id: PlayerId| {
        Box::new(Crusade::new(owner_id))
    });
