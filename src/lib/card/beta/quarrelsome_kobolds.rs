use crate::{
    card::{Card, CardBase, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct QuarrelsomeKobolds {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl QuarrelsomeKobolds {
    pub const NAME: &'static str = "Quarrelsome Kobolds";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                modifiers: vec![],
                types: vec![MinionType::Goblin],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for QuarrelsomeKobolds {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if &state.current_player != self.get_controller_id() {
            return Ok(vec![]);
        }

        let zone = self.get_zone();
        let adjacent_zones = zone.get_adjacent();
        let mut units = vec![];
        for zone in adjacent_zones {
            let units_in_zone = state
                .get_units_in_zone(&zone)
                .iter()
                .filter(|c| c.can_be_targetted_by(state, self.get_controller_id()))
                .map(|c| c.get_id().clone())
                .collect::<Vec<uuid::Uuid>>();
            units.extend(units_in_zone);
        }

        let prompt = "Quarrelsome Kobolds: Pick a unit to deal damage to";
        let picked_unit = pick_card(self.get_controller_id(), &units, state, prompt).await?;
        Ok(vec![Effect::take_damage(
            &picked_unit,
            self.get_id(),
            self.get_power(state)?.unwrap_or(0),
        )])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (QuarrelsomeKobolds::NAME, |owner_id: PlayerId| {
    Box::new(QuarrelsomeKobolds::new(owner_id))
});
