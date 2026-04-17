use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct HeadlessHaunt {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HeadlessHaunt {
    pub const NAME: &'static str = "Headless Haunt";
    pub const DESCRIPTION: &'static str = "Voidwalk\r \r At the start of your turn, Headless Haunt teleports to the top of a random site or void.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Voidwalk],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "AA"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for HeadlessHaunt {
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

    async fn on_turn_start(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        // Only fires on the owner's turn.
        if _state.current_player != *self.get_owner_id() {
            return Ok(vec![]);
        }

        Ok(vec![Effect::MoveCard {
            player_id: *self.get_owner_id(),
            card_id: *self.get_id(),
            from: self.get_zone().clone(),
            to: ZoneQuery::random(Zone::all_realm()),
            tap: false,
            region: self.get_region(_state).clone(),
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HeadlessHaunt::NAME, |owner_id: PlayerId| {
        Box::new(HeadlessHaunt::new(owner_id))
    });
