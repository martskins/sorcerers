use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct FontOfLife {
    card_base: CardBase,
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
                controller_id: owner_id,
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for FontOfLife {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let controller = caster.get_controller_id(state);
        let ally_units: Vec<CardId> = CardQuery::new()
            .units()
            .in_play()
            .controlled_by(&controller)
            .all(state);

        let effects = ally_units
            .into_iter()
            .map(|unit_id| {
                let unit = state.get_card(&unit_id);
                let heal_amount = state.get_body_of_water_size(unit.get_location());
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
static CONSTRUCTOR: (&'static str, CardConstructor) = (FontOfLife::NAME, |owner_id: PlayerId| {
    Box::new(FontOfLife::new(owner_id))
});
