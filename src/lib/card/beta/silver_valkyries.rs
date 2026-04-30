use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

/// **Silver Valkyries** — Elite Minion (6 cost, 4/4)
///
/// Airborne. At the end of your turn, untap all allies here.
#[derive(Debug, Clone)]
pub struct SilverValkyries {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SilverValkyries {
    pub const NAME: &'static str = "Silver Valkyries";
    pub const DESCRIPTION: &'static str =
        "Airborne\n\nAt the end of your turn, untap all allies here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "EE"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SilverValkyries {
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
        if self.get_controller_id(state) != state.current_player {
            return Ok(vec![]);
        }

        let zone = self.get_zone();
        if !zone.is_in_play() {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        Ok(CardQuery::new()
            .units()
            .in_zone(zone)
            .controlled_by(&controller_id)
            .all(state)
            .into_iter()
            .map(|card_id| Effect::UntapCard { card_id })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SilverValkyries::NAME, |owner_id: PlayerId| {
        Box::new(SilverValkyries::new(owner_id))
    });
