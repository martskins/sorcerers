use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone, Default)]
struct PuppetMasterData {
    controller_id: Option<PlayerId>,
    controlled_minions: Vec<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct PuppetMaster {
    unit_base: UnitBase,
    card_base: CardBase,
    data: PuppetMasterData,
}

impl PuppetMaster {
    pub const NAME: &'static str = "Puppet Master";
    pub const DESCRIPTION: &'static str = "Airborne Genesis → Gain control of all tapped minions here until Puppet Master leaves the realm.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "AA"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            data: PuppetMasterData::default(),
        }
    }
}

#[async_trait::async_trait]
impl Card for PuppetMaster {
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

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(data) = data.downcast_ref::<PuppetMasterData>() {
            self.data = data.clone();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for Puppet Master"))
        }
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let self_id = *self.get_id();
        let controlled_minions = CardQuery::new()
            .units()
            .tapped()
            .in_zone(self.get_zone())
            .id_not(&self_id)
            .all(state);
        Ok(vec![Effect::SetCardData {
            card_id: self_id,
            data: Box::new(PuppetMasterData {
                controller_id: Some(controller_id),
                controlled_minions,
            }),
        }])
    }

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() || self.data.controlled_minions.is_empty() {
            return Ok(vec![]);
        }

        let Some(controller_id) = self.data.controller_id else {
            return Ok(vec![]);
        };

        Ok(vec![ContinuousEffect::ControllerOverride {
            controller_id,
            affected_cards: CardQuery::from_ids(self.data.controlled_minions.clone()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PuppetMaster::NAME, |owner_id: PlayerId| {
    Box::new(PuppetMaster::new(owner_id))
});
