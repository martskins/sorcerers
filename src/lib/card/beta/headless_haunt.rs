use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct HeadlessHaunt {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl HeadlessHaunt {
    pub const NAME: &'static str = "Headless Haunt";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Voidwalk],
                types: vec![MinionType::Spirit],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "AA"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for HeadlessHaunt {
    fn get_name(&self) -> &str {
        Self::NAME
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

        Ok(vec![Effect::MoveCard {
            player_id: self.get_owner_id().clone(),
            card_id: self.get_id().clone(),
            from: self.get_zone().clone(),
            to: ZoneQuery::Random {
                id: uuid::Uuid::new_v4(),
                options: Zone::all_realm(),
            },
            tap: false,
            region: Region::Surface,
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (HeadlessHaunt::NAME, |owner_id: PlayerId| {
    Box::new(HeadlessHaunt::new(owner_id))
});
