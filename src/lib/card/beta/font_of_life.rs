use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct FontOfLife {
    pub card_base: CardBase,
}

impl FontOfLife {
    pub const NAME: &'static str = "Font of Life";
    pub const DESCRIPTION: &'static str =
        "Each ally heals an amount equal to the number of sites in its body of water.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for FontOfLife {
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let owner = caster.get_controller_id(state);

        let ally_units: Vec<uuid::Uuid> = CardQuery::new()
            .units()
            .in_play()
            .controlled_by(&owner)
            .all(state);

        let effects = ally_units
            .into_iter()
            .map(|unit_id| {
                let unit = state.get_card(&unit_id);
                let zone = unit.get_zone().clone();
                let heal_amount = state.get_body_of_water_size(&zone).max(1);
                Effect::Heal {
                    card_id: unit_id,
                    amount: heal_amount,
                }
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (FontOfLife::NAME, |owner_id: PlayerId| {
        Box::new(FontOfLife::new(owner_id))
    });
