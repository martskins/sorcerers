use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

/// **Candlemas Monks** — Elite Minion (3 cost, 2/2)
///
/// Deathrite → Proceed to the end phase.
#[derive(Debug, Clone)]
pub struct CandlemasMons {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl CandlemasMons {
    pub const NAME: &'static str = "Candlemas Monks";
    pub const DESCRIPTION: &'static str = "Deathrite → Proceed to the end phase.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
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
impl Card for CandlemasMons {
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

    fn deathrite(&self, state: &State, _from: &Zone) -> Vec<Effect> {
        vec![Effect::EndTurn {
            player_id: self.get_controller_id(state),
        }]
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (CandlemasMons::NAME, |owner_id: PlayerId| {
        Box::new(CandlemasMons::new(owner_id))
    });
