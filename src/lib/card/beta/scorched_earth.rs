use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ScorchedEarth {
    card_base: CardBase,
}

impl ScorchedEarth {
    pub const NAME: &'static str = "Scorched Earth";
    pub const DESCRIPTION: &'static str =
        "Choose any number of sites you control. Destroy each of those sites and everything there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ScorchedEarth {
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
impl Magic for ScorchedEarth {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        // TODO: Add a way to select an undetermined amonut of cards in CardQuery
        let controller_id = state.get_card(caster_id).get_controller_id(state);
        let mut remaining_sites = CardQuery::new()
            .sites()
            .in_play()
            .controlled_by(&controller_id)
            .all(state);
        let mut destroy_effects = vec![];
        loop {
            let Some(site_id) = CardQuery::from_ids(remaining_sites.clone())
                .with_prompt("Choose a site to destroy (or cancel)")
                .with_source_card(*self.get_id())
                .pick(&controller_id, state)
                .await?
            else {
                break;
            };

            let site = state.get_card(&site_id);
            let cards_on_site = CardQuery::new()
                .occupying_site_at_location(site.get_location().clone())
                .all(state);
            destroy_effects.extend(
                cards_on_site
                    .into_iter()
                    .map(|id| Effect::BuryCard { card_id: id }),
            );

            remaining_sites.retain(|id| *id != site_id);
            if remaining_sites.is_empty() {
                break;
            }
        }

        Ok(destroy_effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ScorchedEarth::NAME, |owner_id: PlayerId| {
        Box::new(ScorchedEarth::new(owner_id))
    });
