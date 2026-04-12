use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct ConquerorWorm {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl ConquerorWorm {
    pub const NAME: &'static str = "Conqueror Worm";
    pub const DESCRIPTION: &'static str =
        "At the end of your turn, if no enemy units occupy this site, permanently gain control of it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 8,
                toughness: 8,
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(8, "EE"),
                region: Region::Surface,
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
impl Card for ConquerorWorm {
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
        let controller_id = self.get_controller_id(state);

        // Only trigger at the end of the controller's turn.
        if state.current_player != controller_id {
            return Ok(vec![]);
        }

        // Only act if Conqueror Worm is in play.
        let zone = self.get_zone();
        if !zone.is_in_play() {
            return Ok(vec![]);
        }

        // Check if any enemy units occupy this site.
        let opponent_id = state.get_opponent_id(&controller_id)?;
        let enemy_units = CardQuery::new()
            .units()
            .controlled_by(&opponent_id)
            .in_zone(zone)
            .all(state);

        if !enemy_units.is_empty() {
            return Ok(vec![]);
        }

        // Get the site card at this zone.
        let Some(site) = zone.get_site(state) else {
            return Ok(vec![]);
        };

        // Already controlled by us?
        if site.get_controller_id(state) == controller_id {
            return Ok(vec![]);
        }

        Ok(vec![Effect::SetController {
            card_id: site.get_id().clone(),
            player_id: controller_id,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (ConquerorWorm::NAME, |owner_id: PlayerId| Box::new(ConquerorWorm::new(owner_id)));
