use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Drown {
    pub card_base: CardBase,
}

impl Drown {
    pub const NAME: &'static str = "Drown";
    pub const DESCRIPTION: &'static str = "Submerge target minion or artifact, if able.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Drown {
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
        let controller_id = self.get_controller_id(state);
        let picked_card_id = CardQuery::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .in_regions(vec![Region::Surface])
            .with_prompt("Drown: Pick a minion or artifact to submerge")
            .pick(&controller_id, state, false)
            .await?;
        if picked_card_id.is_none() {
            return Ok(vec![]);
        }
        let picked_card_id = picked_card_id.expect("value to not be None");

        Ok(vec![Effect::SetCardRegion {
            card_id: picked_card_id,
            region: Region::Underwater,
            tap: false,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Drown::NAME, |owner_id: PlayerId| Box::new(Drown::new(owner_id)));
