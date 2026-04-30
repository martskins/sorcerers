use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

/// **Panorama Manticore** — Elite Minion (5 cost, 5/2)
///
/// Airborne, Lethal.
/// At the end of your turn, if you cast a non-fire spell this turn, untap Panorama Manticore.
/// TODO: Implement "if non-fire spell was cast this turn" check.
#[derive(Debug, Clone)]
pub struct PanoramaManticore {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PanoramaManticore {
    pub const NAME: &'static str = "Panorama Manticore";
    pub const DESCRIPTION: &'static str = "Airborne, Lethal\n\nAt the end of your turn, if you cast a non-fire spell this turn, untap Panorama Manticore.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 2,
                abilities: vec![Ability::Airborne, Ability::Lethal],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "FF"),
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
impl Card for PanoramaManticore {
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

        // TODO: Only untap if a non-fire spell was cast this turn.
        // For now, always untap at end of turn as a simplified implementation.
        Ok(vec![Effect::UntapCard {
            card_id: *self.get_id(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PanoramaManticore::NAME, |owner_id: PlayerId| {
        Box::new(PanoramaManticore::new(owner_id))
    });
