use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct CraveGolem {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl CraveGolem {
    pub const NAME: &'static str = "Crave Golem";
    pub const DESCRIPTION: &'static str = "At the start of each player's turn, Crave Golem attacks a random minion within its range of motion, or takes a step toward the closest minion if it can't.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Automaton],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(4),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CraveGolem {
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
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        // TODO: This implementation needs reviewing. The possible targets for the strike are not
        // quite right in that it currently incldes minions that are not attackable or are airborne,
        // etc.
        let controller_id = self.get_controller_id(state);
        let target_id = CardQuery::new()
            .minions()
            .within_range_of(self.get_id())
            .id_not_in(vec![self.get_id().clone()])
            .randomised()
            .count(1)
            .pick(&controller_id, state, false)
            .await?;
        match target_id {
            Some(card_id) => {
                return Ok(vec![Effect::Attack {
                    attacker_id: self.get_id().clone(),
                    defender_id: card_id,
                }]);
            }
            None => {
                // BFS to find the closest zone with a minion, then move one step toward it.
                let self_zone = self.get_zone().clone();
                let mut visited: Vec<Zone> = vec![];
                let mut queue: std::collections::VecDeque<(Zone, Zone)> =
                    std::collections::VecDeque::new();

                for adj in self_zone.get_adjacent() {
                    if adj.is_in_play() {
                        queue.push_back((adj.clone(), adj.clone()));
                    }
                }
                visited.push(self_zone.clone());

                let mut first_step: Option<Zone> = None;

                'bfs: while let Some((current, step_from_self)) = queue.pop_front() {
                    if visited.contains(&current) {
                        continue;
                    }
                    visited.push(current.clone());

                    let has_minion = state
                        .get_minions_in_zone(&current)
                        .iter()
                        .any(|c| c.get_id() != self.get_id());

                    if has_minion {
                        first_step = Some(step_from_self);
                        break 'bfs;
                    }

                    for adj in current.get_adjacent() {
                        if adj.is_in_play() && !visited.contains(&adj) {
                            queue.push_back((adj.clone(), step_from_self.clone()));
                        }
                    }
                }

                if let Some(target_zone) = first_step {
                    if target_zone.get_site(state).is_some() {
                        return Ok(vec![Effect::MoveCard {
                            card_id: self.get_id().clone(),
                            to: ZoneQuery::from_zone(target_zone),
                            player_id: self.get_controller_id(state),
                            from: self_zone,
                            tap: true,
                            region: self.get_region(state).clone(),
                            through_path: None,
                        }]);
                    }
                }
            }
        }

        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (CraveGolem::NAME, |owner_id: PlayerId| {
        Box::new(CraveGolem::new(owner_id))
    });
