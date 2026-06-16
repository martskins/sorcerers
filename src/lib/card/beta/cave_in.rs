use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct CaveIn {
    card_base: CardBase,
}

impl CaveIn {
    pub const NAME: &'static str = "Cave-In";
    pub const DESCRIPTION: &'static str =
        "Burrow all minions and artifacts occupying target land site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "E"),
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
impl Card for CaveIn {
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
impl Magic for CaveIn {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let player_id = self.get_controller_id(state);
        let Some(picked_site_id) = CardQuery::new()
            .land_sites()
            .with_source_card(*self.get_id())
            .with_prompt("Pick a land site")
            .pick(&player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };
        let picked_site = state.get_card(&picked_site_id);
        let minions_and_artifacts = CardQuery::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .occupying_site_at_location(self.get_location().clone())
            .all(state);

        Ok(minions_and_artifacts
            .iter()
            .map(|card_id| Effect::MoveCard {
                player_id: self.get_controller_id(state),
                card_id: *card_id,
                from: picked_site.get_location().clone(),
                to: picked_site
                    .get_location()
                    .with_region(Region::Underground)
                    .into(),
                tap: false,
                through_path: None,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (CaveIn::NAME, |owner_id: PlayerId| {
    Box::new(CaveIn::new(owner_id))
});
