use crate::{
    card::{AdditionalCost, Card, CardBase, Cost, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, Direction, PlayerId, pick_direction},
    query::ZoneQuery,
    state::{CardQuery, State},
};

const CARDINAL_DIRECTIONS: &[Direction] = &[Direction::Up, Direction::Down, Direction::Left, Direction::Right];

#[derive(Debug, Clone)]
struct TapMoveAndStrike;

#[async_trait::async_trait]
impl ActivatedAbility for TapMoveAndStrike {
    fn get_name(&self) -> String {
        "Tap → Move three steps, striking each untapped unit along the way".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let bull_demons = state.get_card(card_id);
        let start_zone = bull_demons.get_zone().clone();

        let direction = pick_direction(
            player_id,
            CARDINAL_DIRECTIONS,
            state,
            "Bull Demons of Adum: Choose a cardinal direction to charge",
        )
        .await?;

        let mut effects: Vec<Effect> = vec![];
        let mut path: Vec<Zone> = vec![];
        let mut current_zone = start_zone.clone();

        for _ in 0..3 {
            match current_zone.zone_in_direction(&direction, 1) {
                Some(next_zone) => {
                    // Strike each untapped unit in the destination zone.
                    let targets = CardQuery::new()
                        .units()
                        .untapped()
                        .id_not_in(vec![card_id.clone()])
                        .in_zone(&next_zone)
                        .all(state);

                    for target_id in targets {
                        effects.push(Effect::TakeDamage {
                            card_id: target_id,
                            from: card_id.clone(),
                            damage: bull_demons
                                .get_power(state)?
                                .ok_or_else(|| anyhow::anyhow!("Bull Demons has no power"))?,
                        });
                    }

                    path.push(next_zone.clone());
                    current_zone = next_zone;
                }
                None => break,
            }
        }

        if let Some(final_zone) = path.last() {
            effects.push(Effect::MoveCard {
                player_id: player_id.clone(),
                card_id: card_id.clone(),
                from: start_zone,
                to: ZoneQuery::from_zone(final_zone.clone()),
                tap: false,
                region: Region::Surface,
                through_path: Some(path),
            });
        }

        Ok(effects)
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(
            CardQuery::from_id(card_id.clone()).untapped(),
        )))
    }
}

#[derive(Debug, Clone)]
pub struct BullDemonsOfAdum {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl BullDemonsOfAdum {
    pub const NAME: &'static str = "Bull Demons of Adum";
    pub const DESCRIPTION: &'static str = "Tap → Move three steps in a cardinal direction. When Bull Demons of Adum enter each location, they strike each untapped unit there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![],
                types: vec![MinionType::Demon],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "FF"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for BullDemonsOfAdum {
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

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(TapMoveAndStrike)])
    }

    async fn on_visit_zone(&self, state: &State, to: &Zone) -> anyhow::Result<Vec<Effect>> {
        Ok(CardQuery::new()
            .units()
            .untapped()
            .in_zone(to)
            .all(state)
            .into_iter()
            .map(|target_id| Effect::RangedStrike {
                attacker_id: self.get_id().clone(),
                defender_id: target_id.clone(),
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (BullDemonsOfAdum::NAME, |owner_id: PlayerId| {
    Box::new(BullDemonsOfAdum::new(owner_id))
});
