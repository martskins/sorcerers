use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
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
                modifiers: vec![Modifier::Voidwalk],
                types: vec![MinionType::Spirit],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                controller_id: owner_id.clone(),
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

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    async fn on_turn_start(&self, _state: &State) -> Vec<Effect> {
        if !self.get_zone().is_in_realm() {
            return vec![];
        }

        vec![Effect::MoveCard {
            player_id: self.get_owner_id().clone(),
            card_id: self.get_id().clone(),
            from: self.get_zone().clone(),
            to: ZoneQuery::Random {
                id: uuid::Uuid::new_v4(),
                options: Zone::all_realm(),
            },
            tap: false,
            plane: Plane::Surface,
            through_path: None,
        }]
    }
}
