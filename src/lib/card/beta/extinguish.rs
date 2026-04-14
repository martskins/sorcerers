use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{Element, PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Extinguish {
    pub card_base: CardBase,
}

impl Extinguish {
    pub const NAME: &'static str = "Extinguish";
    pub const DESCRIPTION: &'static str =
        "Banish all fire minions and fire auras occupying target site up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
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
impl Card for Extinguish {
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
        let zones = caster.get_zones_within_steps(state, 2);

        let picked_zone = pick_zone(
            self.get_owner_id(),
            &zones,
            state,
            false,
            "Extinguish: Pick a target site",
        )
        .await?;

        let targets = CardQuery::new()
            .with_affinity(Element::Fire)
            .card_types(vec![CardType::Minion, CardType::Aura])
            .in_zone(&picked_zone)
            .all(state);

        Ok(targets
            .into_iter()
            .map(|card_id| Effect::BanishCard {
                card_id,
                from: picked_zone.clone(),
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Extinguish::NAME, |owner_id: PlayerId| {
        Box::new(Extinguish::new(owner_id))
    });
