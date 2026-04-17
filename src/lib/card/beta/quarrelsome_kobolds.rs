use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct QuarrelsomeKobolds {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl QuarrelsomeKobolds {
    pub const NAME: &'static str = "Quarrelsome Kobolds";
    pub const DESCRIPTION: &'static str = "At the end of your turn, Quarrelsome Kobolds strike themselves or another target adjacent unit.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Goblin],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
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
impl Card for QuarrelsomeKobolds {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if state.current_player != self.get_controller_id(state) {
            return Ok(vec![]);
        }

        let zone = self.get_zone();
        let adjacent_zones = zone.get_adjacent();
        let mut units = vec![];
        let player_id = self.get_controller_id(state);
        for zone in adjacent_zones {
            let units_in_zone = CardQuery::new()
                .units()
                .in_zone(&zone)
                .can_be_targeted_by_player(&player_id)
                .all(state);
            units.extend(units_in_zone);
        }

        let prompt = "Quarrelsome Kobolds: Pick a unit to deal damage to";
        let picked_unit = pick_card(self.get_controller_id(state), &units, state, prompt).await?;
        Ok(vec![Effect::take_damage(
            &picked_unit,
            self.get_id(),
            self.get_power(state)?.unwrap_or(0),
        )])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (QuarrelsomeKobolds::NAME, |owner_id: PlayerId| {
        Box::new(QuarrelsomeKobolds::new(owner_id))
    });
