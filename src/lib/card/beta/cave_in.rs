use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{Element, PlayerId, pick_card},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct CaveIn {
    pub card_base: CardBase,
}

impl CaveIn {
    pub const NAME: &'static str = "Cave-In";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "E"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CaveIn {
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
        let valid_targets = state
            .cards
            .iter()
            .filter(|c| c.is_site())
            .filter(|c| {
                c.get_site()
                    .expect("site card has no site base")
                    .get_elements(state)
                    .unwrap_or_default()
                    .contains(&Element::Earth)
            })
            .map(|c| c.get_id())
            .cloned()
            .collect::<Vec<_>>();

        let picked_site_id = pick_card(
            &self.get_controller_id(),
            &valid_targets,
            state,
            "Cave-In: Pick a target site",
        )
        .await?;

        let picked_site = state.get_card(&picked_site_id);
        let minions_and_artifacts: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.get_zone() == picked_site.get_zone())
            .filter(|c| c.is_minion() || c.is_artifact())
            .map(|c| c.get_id())
            .cloned()
            .collect();

        Ok(minions_and_artifacts
            .iter()
            .map(|card_id| Effect::MoveCard {
                player_id: self.get_controller_id().clone(),
                card_id: card_id.clone(),
                from: picked_site.get_zone().clone(),
                to: ZoneQuery::Specific {
                    id: uuid::Uuid::new_v4(),
                    zone: picked_site.get_zone().clone(),
                },
                tap: false,
                plane: Plane::Underground,
                through_path: None,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (CaveIn::NAME, |owner_id: PlayerId| Box::new(CaveIn::new(owner_id)));