use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct Bury {
    card_base: CardBase,
}

impl Bury {
    pub const NAME: &'static str = "Bury";
    pub const DESCRIPTION: &'static str = "Burrow target minion or artifact, if able.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Bury {
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
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let valid_targets = state
            .cards
            .iter()
            .filter(|c| c.is_minion() || c.is_artifact())
            .filter(|c| c.get_region(state) >= &Region::Surface)
            .map(|c| c.get_id())
            .cloned()
            .collect::<Vec<_>>();
        let picked_card_id = pick_card(
            &self.get_controller_id(state),
            &valid_targets,
            state,
            "Bury: Pick a minion or artifact to bury",
        )
        .await?;
        let picked_card = state.get_card(&picked_card_id);

        Ok(vec![Effect::MoveCard {
            player_id: self.get_controller_id(state),
            card_id: picked_card_id,
            from: picked_card.get_zone().clone(),
            to: ZoneQuery::from_zone(picked_card.get_zone().clone()),
            tap: false,
            region: Region::Underground,
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Bury::NAME, |owner_id: PlayerId| {
    Box::new(Bury::new(owner_id))
});
