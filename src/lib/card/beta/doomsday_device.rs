use crate::{
    card::{Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct DoomsdayDevice {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
    doom_counters: u8,
}

impl DoomsdayDevice {
    pub const NAME: &'static str = "Doomsday Device";
    pub const DESCRIPTION: &'static str = "Doomsday Device enters the realm with 6 counters. At the end of each player's turn, remove a counter. When the last is removed, it detonates! Deals damage to each unit at affected locations.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Device],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(4),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            doom_counters: 0,
        }
    }
}

impl Artifact for DoomsdayDevice {}

#[async_trait::async_trait]
impl Card for DoomsdayDevice {
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

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(val) = data.downcast_ref::<u8>() {
            self.doom_counters = *val;
        }
        Ok(())
    }

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::SetCardData {
            card_id: self.get_id().clone(),
            data: Box::new(6u8),
        }])
    }

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        if self.doom_counters == 0 {
            return Ok(vec![]);
        }

        let self_id = self.get_id().clone();

        if self.doom_counters == 1 {
            // Trigger the explosion.
            let explosion_zones: Vec<Zone> = std::iter::once(self.get_zone().clone())
                .chain(self.get_zone().get_nearby())
                .collect();

            let mut effects: Vec<Effect> = CardQuery::new()
                .units()
                .in_zones(&explosion_zones)
                .all(state)
                .into_iter()
                .map(|id| Effect::TakeDamage {
                    card_id: id,
                    from: self_id.clone(),
                    damage: 6,
                    is_strike: false,
                })
                .collect();

            effects.push(Effect::BuryCard { card_id: self_id });
            return Ok(effects);
        }

        Ok(vec![Effect::SetCardData {
            card_id: self_id,
            data: Box::new(self.doom_counters - 1),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (DoomsdayDevice::NAME, |owner_id: PlayerId| {
    Box::new(DoomsdayDevice::new(owner_id))
});
