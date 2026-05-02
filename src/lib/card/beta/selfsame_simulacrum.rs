use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct SelfsameSimulacrum {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SelfsameSimulacrum {
    pub const NAME: &'static str = "Selfsame Simulacrum";
    pub const DESCRIPTION: &'static str = "May be summoned as a basic copy of a nearby minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 0,
                types: vec![MinionType::Fairy],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
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
impl Card for SelfsameSimulacrum {
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
        if let Some(unit_base) = data.downcast_ref::<UnitBase>() {
            self.unit_base = unit_base.clone();
            self.unit_base.damage = 0;
            self.unit_base.tapped = false;
            self.unit_base.carried_by = None;
            self.unit_base.power_counters.clear();
            self.unit_base.ability_counters.clear();
        }
        Ok(())
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let targets = CardQuery::new()
            .minions()
            .near_to(self.get_zone())
            .id_not(self.get_id())
            .all(state);
        if targets.is_empty() {
            return Ok(vec![]);
        }

        let chosen_id = pick_card(
            &controller_id,
            &targets,
            state,
            "Selfsame Simulacrum: Pick a nearby minion to copy",
        )
        .await?;
        let mut copied = state
            .get_card(&chosen_id)
            .get_unit_base()
            .cloned()
            .ok_or(anyhow::anyhow!("chosen minion has no unit base"))?;
        copied.damage = 0;
        copied.tapped = false;
        copied.carried_by = None;
        copied.power_counters.clear();
        copied.ability_counters.clear();

        Ok(vec![Effect::SetCardData {
            card_id: *self.get_id(),
            data: Box::new(copied),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SelfsameSimulacrum::NAME, |owner_id: PlayerId| {
        Box::new(SelfsameSimulacrum::new(owner_id))
    });
