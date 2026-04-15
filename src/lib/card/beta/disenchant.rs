use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Disenchant {
    card_base: CardBase,
}

impl Disenchant {
    pub const NAME: &'static str = "Disenchant";
    pub const DESCRIPTION: &'static str =
        "Destroy all auras and artifacts at target location up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
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
impl Card for Disenchant {
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
        let caster = state.get_card(caster_id);

        let zones_within_two = caster.get_zones_within_steps(state, 2);
        let target_zone = ZoneQuery::from_options(
            zones_within_two,
            Some("Disenchant: Pick a zone to disenchant".to_string()),
        )
        .pick(&controller_id, state)
        .await?;

        let artifacts_and_auras = CardQuery::new()
            .card_types(vec![CardType::Aura, CardType::Artifact])
            .in_zone(&target_zone)
            .all(state);

        Ok(artifacts_and_auras
            .into_iter()
            .map(|id| Effect::BuryCard { card_id: id })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Disenchant::NAME, |owner_id: PlayerId| {
        Box::new(Disenchant::new(owner_id))
    });
