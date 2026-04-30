use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct GreatOldOne {
    unit_base: UnitBase,
    card_base: CardBase,
    flooded: bool,
}

impl GreatOldOne {
    pub const NAME: &'static str = "Great Old One";
    pub const DESCRIPTION: &'static str =
        "Submerge\r \r Genesis → Permanently flood the entire realm, including voids.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 16,
                toughness: 16,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Monster],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(8, "WWW"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            flooded: false,
        }
    }
}

#[async_trait::async_trait]
impl Card for GreatOldOne {
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

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::SetCardData {
            card_id: *self.get_id(),
            data: Box::new(true),
        }])
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(b) = data.downcast_ref::<bool>() {
            self.flooded = *b;
        }
        Ok(())
    }

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.flooded {
            return Ok(vec![]);
        }
        let all_sites = CardQuery::new().sites().all(state);
        if all_sites.is_empty() {
            return Ok(vec![]);
        }
        Ok(vec![ContinuousEffect::FloodSites {
            affected_sites: CardQuery::from_ids(all_sites),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GreatOldOne::NAME, |owner_id: PlayerId| {
    Box::new(GreatOldOne::new(owner_id))
});
