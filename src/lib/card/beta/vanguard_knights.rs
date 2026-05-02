use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct VanguardKnights {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl VanguardKnights {
    pub const NAME: &'static str = "Vanguard Knights";
    pub const DESCRIPTION: &'static str =
        "Vanguard Knights have +2 power if they alone are the furthest forward of your units.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn front_score(card: &dyn Card, state: &State) -> Option<u8> {
        match card.get_zone() {
            Zone::Realm(square) => Some(*square),
            Zone::Intersection(squares) => {
                if card.get_controller_id(state) == state.player_one {
                    squares.iter().copied().max()
                } else {
                    squares.iter().copied().min()
                }
            }
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl Card for VanguardKnights {
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

    fn get_power(&self, state: &State) -> anyhow::Result<Option<u16>> {
        let Some(mut power) = self.base_get_power(state) else {
            return Ok(None);
        };

        if !self.get_zone().is_in_play() {
            return Ok(Some(power));
        }

        let controller_id = self.get_controller_id(state);
        let scores: Vec<(uuid::Uuid, u8)> = CardQuery::new()
            .units()
            .controlled_by(&controller_id)
            .in_play()
            .all(state)
            .into_iter()
            .filter_map(|id| Self::front_score(state.get_card(&id), state).map(|score| (id, score)))
            .collect();

        if let Some(self_score) = Self::front_score(self, state) {
            let furthest = if controller_id == state.player_one {
                scores.iter().map(|(_, score)| *score).max()
            } else {
                scores.iter().map(|(_, score)| *score).min()
            };

            if furthest == Some(self_score)
                && scores
                    .iter()
                    .filter(|(_, score)| *score == self_score)
                    .count()
                    == 1
            {
                power += 2;
            }
        }

        Ok(Some(power))
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (VanguardKnights::NAME, |owner_id: PlayerId| {
        Box::new(VanguardKnights::new(owner_id))
    });
