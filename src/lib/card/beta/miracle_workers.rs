use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase,
        Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct MiracleWorkers {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MiracleWorkers {
    pub const NAME: &'static str = "Miracle Workers";
    pub const DESCRIPTION: &'static str =
        "Genesis → You may return a minion that died this turn from your cemetery to your hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MiracleWorkers {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let current_turn = state.turns;

        // Find allied minions that died this turn (BuryCard in the current turn's effect log).
        let died_this_turn: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_minion() && c.get_zone() == &Zone::Cemetery)
            .filter(|c| c.get_owner_id() == &controller_id)
            .filter(|c| {
                state.effect_log.iter().any(|le| {
                    le.turn == current_turn
                        && matches!(*le.effect, Effect::BuryCard { card_id } if card_id == *c.get_id())
                })
            })
            .map(|c| *c.get_id())
            .collect();

        if died_this_turn.is_empty() {
            return Ok(vec![]);
        }

        let Some(chosen) = CardQuery::new()
            .in_zone(&Zone::Cemetery)
            .controlled_by(&controller_id)
            .with_prompt("Miracle Workers: Return a minion that died this turn to your hand?")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        if !died_this_turn.contains(&chosen) {
            return Ok(vec![]);
        }

        Ok(vec![Effect::MoveCard {
            player_id: controller_id,
            card_id: chosen,
            from: Zone::Cemetery,
            to: crate::query::ZoneQuery::from_zone(Zone::Hand),
            tap: false,
            region: crate::card::Region::Surface,
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MiracleWorkers::NAME, |owner_id: PlayerId| {
        Box::new(MiracleWorkers::new(owner_id))
    });
