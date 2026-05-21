use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ImperialRoad {
    site_base: SiteBase,
    card_base: CardBase,
}

impl ImperialRoad {
    pub const NAME: &'static str = "Imperial Road";
    pub const DESCRIPTION: &'static str =
        "Genesis → Target opponent, then you, may play a site adjacent to this one.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
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
impl Site for ImperialRoad {}

impl ResourceProvider for ImperialRoad {}

#[async_trait::async_trait]
impl Card for ImperialRoad {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;
        let my_zone = self.get_zone().clone();
        let adjacent_zones: Vec<Zone> = my_zone
            .get_adjacent()
            .into_iter()
            .filter(|z| z.get_site(state).is_none())
            .collect();

        if adjacent_zones.is_empty() {
            return Ok(vec![]);
        }

        let mut effects = vec![];

        for player_id in [opponent_id, controller_id] {
            let sites_in_atlasbook: Vec<uuid::Uuid> = state
                .cards
                .values()
                .filter(|c| c.is_site())
                .filter(|c| c.get_zone() == &Zone::Atlasbook)
                .filter(|c| c.get_controller_id(state) == player_id)
                .map(|c| *c.get_id())
                .collect();

            if sites_in_atlasbook.is_empty() {
                continue;
            }

            let Some(chosen_site) = CardQuery::from_ids(sites_in_atlasbook)
                .with_prompt("Play an adjacent site?").with_source_card(*self.get_id())
                .pick(&player_id, state, true)
                .await?
            else {
                continue;
            };

            let valid_zones: Vec<Zone> = adjacent_zones
                .iter()
                .filter(|&z| z.get_site(state).is_none())
                .cloned()
                .collect();

            if valid_zones.is_empty() {
                continue;
            }

            let zone = pick_zone(
                &player_id,
                &valid_zones,
                state,
                true,
                "Imperial Road: Pick adjacent zone to place site",
            )
            .await?;

            let avatar_id = state.get_player_avatar_id(&player_id)?;
            effects.push(Effect::PlayCard {
                player_id,
                card_id: chosen_site,
                zone: zone.into(),
                // we pass avatar_id as the caster just to comply with the required parameters, but
                // no caster_id is actually needed here, since sites don't need one.
                spellcaster: avatar_id,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (ImperialRoad::NAME, |owner_id: PlayerId| {
    Box::new(ImperialRoad::new(owner_id))
});
