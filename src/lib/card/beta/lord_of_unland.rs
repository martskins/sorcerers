use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct LordOfUnland {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl LordOfUnland {
    pub const NAME: &'static str = "Lord of Unland";
    pub const DESCRIPTION: &'static str =
        "Submerge\r \r Allied minions sharing the same body of water have +1 power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Merfolk],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WWW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    /// Returns all zones connected to `start` through water sites (BFS).
    fn connected_water_zones(start: &Zone, state: &State) -> Vec<Zone> {
        let mut visited: Vec<Zone> = vec![start.clone()];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start.clone());

        while let Some(zone) = queue.pop_front() {
            for adj in zone.get_adjacent() {
                if visited.contains(&adj) {
                    continue;
                }
                let is_water = adj
                    .get_site(state)
                    .and_then(|s| s.is_water_site(state).ok())
                    .unwrap_or(false);
                if is_water {
                    visited.push(adj.clone());
                    queue.push_back(adj);
                }
            }
        }

        visited
    }
}

#[async_trait::async_trait]
impl Card for LordOfUnland {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let is_at_water = self
            .get_zone()
            .get_site(state)
            .and_then(|s| s.is_water_site(state).ok())
            .unwrap_or(false);

        if !is_at_water {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let water_zones = Self::connected_water_zones(self.get_zone(), state);

        let allies: Vec<uuid::Uuid> = CardQuery::new()
            .minions()
            .in_zones(&water_zones)
            .controlled_by(&controller_id)
            .id_not(self.get_id())
            .all(state);

        if allies.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::ModifyPower {
            power_diff: 1,
            affected_cards: CardQuery::from_ids(allies),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (LordOfUnland::NAME, |owner_id: PlayerId| {
    Box::new(LordOfUnland::new(owner_id))
});
