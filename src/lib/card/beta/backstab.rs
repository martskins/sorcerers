use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct Backstab {
    pub card_base: CardBase,
}

impl Backstab {
    pub const NAME: &'static str = "Backstab";

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
        let mover_candidates = CardMatcher::new()
            .with_card_types(vec![CardType::Minion])
            .with_region(&caster_region)
            .resolve_ids(state);
        if mover_candidates.is_empty() {
            return Ok(vec![]);
        }

        let mover_id = pick_card(
            &controller_id,
            &mover_candidates,
            state,
            "Backstab: Pick a minion to move",
        )
        .await?;
        let mover = state.get_card(&mover_id);
        let target_ids = CardMatcher::new()
            .with_card_types(vec![CardType::Minion])
            .with_region(&caster_region)
            .with_tapped(true)
            .in_zones(&mover.get_zone().get_adjacent())
            .with_id_not_in(vec![mover_id.clone()])
            .resolve_ids(state);
        if target_ids.is_empty() {
            return Ok(vec![]);
        }

        let target_id = pick_card(
            &controller_id,
            &target_ids,
            state,
            "Backstab: Pick a tapped minion to strike",
        )
        .await?;

        Ok(vec![Effect::Strike {
            attacker_id: mover_id,
            defender_id: target_id,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Backstab::NAME, |owner_id: PlayerId| Box::new(Backstab::new(owner_id)));
