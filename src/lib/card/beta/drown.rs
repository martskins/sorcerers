use crate::{
    card::{Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct Drown {
    pub card_base: CardBase,
}

impl Drown {
    pub const NAME: &'static str = "Drown";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "W"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Drown {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let possible_targets = CardMatcher::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .in_regions(vec![Region::Surface])
            .resolve_ids(state);
        if possible_targets.is_empty() {
            return Ok(vec![]);
        }

        let prompt = "Drown: Pick a minion or artifact to submerge";
        let picked_card = pick_card(self.get_controller_id(state), &possible_targets, state, prompt).await?;
        Ok(vec![Effect::Submerge { card_id: picked_card }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Drown::NAME, |owner_id: PlayerId| Box::new(Drown::new(owner_id)));