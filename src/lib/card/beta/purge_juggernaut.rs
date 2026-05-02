use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct PurgeJuggernaut {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PurgeJuggernaut {
    pub const NAME: &'static str = "Purge Juggernaut";
    pub const DESCRIPTION: &'static str = "At the start of your turn, Purge Juggernaut taps and moves to an adjacent location. Kill all other minions there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                types: vec![MinionType::Automaton],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(6),
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
impl Card for PurgeJuggernaut {
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

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        let self_id = *self.get_id();
        let adjacent_locations: Vec<Zone> = self
            .get_zone()
            .get_adjacent()
            .into_iter()
            .filter(|z| z.get_site(state).is_some())
            .collect();
        if adjacent_locations.is_empty() {
            return Ok(vec![]);
        }

        let target_zone = pick_zone(
            &controller_id,
            &adjacent_locations,
            state,
            false,
            "Purge Juggernaut: Pick an adjacent location",
        )
        .await?;
        let killed_units: Vec<Effect> = CardQuery::new()
            .units()
            .in_zone(&target_zone)
            .id_not(self.get_id())
            .all(state)
            .into_iter()
            .map(|unit_id| Effect::KillMinion {
                card_id: unit_id,
                killer_id: self_id,
            })
            .collect();
        let mut effects = vec![Effect::MoveCard {
            player_id: controller_id,
            card_id: self_id,
            from: self.get_zone().clone(),
            to: target_zone.clone().into(),
            tap: true,
            region: self.get_region(state).clone(),
            through_path: None,
        }];
        effects.extend(killed_units);
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PurgeJuggernaut::NAME, |owner_id: PlayerId| {
        Box::new(PurgeJuggernaut::new(owner_id))
    });
