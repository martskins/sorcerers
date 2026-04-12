use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Backstab {
    pub card_base: CardBase,
}

impl Backstab {
    pub const NAME: &'static str = "Backstab";
    pub const DESCRIPTION: &'static str =
        "Target minion moves to an adjacent location, if needed, to strike another target tapped minion there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
                region: Region::Surface,
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
impl Card for Backstab {
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
        let controller_id = self.get_controller_id(state);
        let caster_region = state.get_card(caster_id).get_region(state).clone();
        let mover_id = CardQuery::new()
            .card_types(vec![CardType::Minion])
            .in_region(&caster_region)
            .with_prompt("Backstab: Pick a minion to move")
            .pick(&controller_id, state, false)
            .await?;
        if mover_id.is_none() {
            return Ok(vec![]);
        }
        let mover_id = mover_id.expect("mover_id to not be None");

        let mover = state.get_card(&mover_id);
        let target_id = CardQuery::new()
            .card_types(vec![CardType::Minion])
            .in_region(&caster_region)
            .tapped()
            .in_zones(&mover.get_zone().get_adjacent())
            .id_not_in(vec![mover_id.clone()])
            .with_prompt("Backstab: Pick a tapped minion to strike")
            .pick(&controller_id, state, false)
            .await?;
        if target_id.is_none() {
            return Ok(vec![]);
        }
        let target_id = target_id.expect("target_id to not be None");

        Ok(vec![Effect::Strike {
            attacker_id: mover_id,
            defender_id: target_id,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Backstab::NAME, |owner_id: PlayerId| Box::new(Backstab::new(owner_id)));
