use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Mirage {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Mirage {
    pub const NAME: &'static str = "Mirage";
    pub const DESCRIPTION: &'static str = "When played, you may return a site in play you own to your hand to play this site in its place.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
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
impl Site for Mirage {}

impl ResourceProvider for Mirage {}

#[async_trait::async_trait]
impl Card for Mirage {
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

    fn get_valid_play_locations(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Location>> {
        let mut locations = self.base_get_valid_play_locations(state, player_id, caster_id)?;
        locations.extend(
            CardQuery::new()
                .sites()
                .owned_by(player_id)
                .in_play()
                .id_not(*self.get_id())
                .all(state)
                .into_iter()
                .map(|site_id| state.get_card(&site_id).get_location().clone()),
        );
        locations.sort();
        locations.dedup();
        Ok(locations)
    }

    async fn play_mechanic(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Effect>> {
        let locations = self.get_valid_play_locations(state, player_id, caster_id)?;
        let location = LocationQuery::from_locations(locations)
            .with_prompt("Pick a zone to play the site")
            .with_source_card(*self.get_id())
            .pick(player_id, state)
            .await?;
        self.play_mechanic_at_location(state, player_id, caster_id, &location)
            .await
    }

    async fn play_mechanic_at_location(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
        location: &Location,
    ) -> anyhow::Result<Vec<Effect>> {
        if let Some(site) = location.get_site(state)
            && site.get_owner_id() == player_id
            && site.get_id() != self.get_id()
        {
            return Ok(vec![
                Effect::SetCardZone {
                    card_id: *site.get_id(),
                    zone: Zone::Hand,
                },
                Effect::PlayCard {
                    player_id: *player_id,
                    card_id: *self.get_id(),
                    location: location.clone(),
                    spellcaster: *caster_id,
                },
            ]);
        }

        Ok(vec![Effect::PlayCard {
            player_id: *player_id,
            card_id: *self.get_id(),
            location: location.clone(),
            spellcaster: *caster_id,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Mirage::NAME, |owner_id: PlayerId| {
    Box::new(Mirage::new(owner_id))
});
