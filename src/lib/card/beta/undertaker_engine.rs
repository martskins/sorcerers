use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct UndertakerEngine {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl UndertakerEngine {
    pub const NAME: &'static str = "Undertaker Engine";
    pub const DESCRIPTION: &'static str =
        "At the end of your turn, toggle the region of all units here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                types: vec![MinionType::Automaton],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(7),
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
impl Card for UndertakerEngine {
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
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        let self_id = *self.get_id();
        let effects: Vec<Effect> = CardQuery::new()
            .units()
            .in_zone(self.get_zone())
            .id_not(&self_id)
            .all(state)
            .into_iter()
            .map(|unit_id| {
                let unit = state.get_card(&unit_id);
                let new_region = if let Some(ub) = unit.get_unit_base() {
                    if ub.region == Region::Surface {
                        Region::Underground
                    } else {
                        Region::Surface
                    }
                } else {
                    Region::Surface
                };
                Effect::SetCardRegion {
                    card_id: unit_id,
                    region: new_region,
                    tap: false,
                }
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (UndertakerEngine::NAME, |owner_id: PlayerId| {
        Box::new(UndertakerEngine::new(owner_id))
    });
