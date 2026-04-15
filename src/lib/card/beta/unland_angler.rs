use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct UnlandAngler {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl UnlandAngler {
    pub const NAME: &'static str = "Unland Angler";
    pub const DESCRIPTION: &'static str = "Submerge\r \r At the start of your turn, if Unland Angler is submerged, force each enemy minion atop adjacent sites to take a step toward this one.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "WW"),
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
impl Card for UnlandAngler {
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

        if self.get_region(state) != &Region::Underwater {
            return Ok(vec![]);
        }

        let opponent_id = state.get_opponent_id(&controller_id)?;
        let effects = CardQuery::new()
            .minions()
            .controlled_by(&opponent_id)
            .adjacent_to(self.get_zone())
            .all(state)
            .into_iter()
            .map(|minion_id| {
                let minion = state.get_card(&minion_id);
                Effect::MoveCard {
                    player_id: controller_id.clone(),
                    card_id: minion_id,
                    from: minion.get_zone().clone(),
                    to: ZoneQuery::from_zone(self.get_zone().clone()),
                    tap: minion.is_tapped(),
                    region: minion.get_region(state).clone(),
                    through_path: None,
                }
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (UnlandAngler::NAME, |owner_id: PlayerId| {
        Box::new(UnlandAngler::new(owner_id))
    });
