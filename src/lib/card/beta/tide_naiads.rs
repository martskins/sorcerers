use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::PlayerId,
    state::{CardMatcher, ContinousEffect, State},
};

#[derive(Debug, Clone)]
pub struct TideNaiads {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl TideNaiads {
    pub const NAME: &'static str = "Tide Naiads";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Spirit],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "WW"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for TideNaiads {
    fn get_name(&self) -> &str {
        Self::NAME
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

    async fn get_continuos_effects(&self, state: &State) -> anyhow::Result<Vec<ContinousEffect>> {
        let site_id = self.get_zone().get_site(state).map(|site| site.get_id()).cloned();

        Ok(vec![ContinousEffect::FloodSites {
            affected_sites: CardMatcher {
                id: site_id,
                ..Default::default()
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (TideNaiads::NAME, |owner_id: PlayerId| {
    Box::new(TideNaiads::new(owner_id))
});
