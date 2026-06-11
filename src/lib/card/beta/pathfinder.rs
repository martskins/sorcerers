use crate::prelude::*;

#[derive(Debug, Clone)]
struct PathfinderSitewalk;

#[async_trait::async_trait]
impl ActivatedAbility for PathfinderSitewalk {
    fn get_name(&self) -> String {
        "Play top site and move there".to_string()
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    fn can_activate(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let Some(site_id) = state.get_player_deck(player_id)?.peek_site() else {
            return Ok(false);
        };
        let pathfinder = state.get_card(card_id);
        let site = state.get_card(site_id);
        let valid_play_zones = site.get_valid_play_zones(state, player_id, card_id)?;
        Ok(pathfinder
            .get_zone()
            .get_adjacent_locations(state)
            .into_iter()
            .any(|zone| valid_play_zones.contains(&zone)))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let mut deck = state.get_player_deck(player_id)?.clone();
        let Some(site_id) = deck.sites.pop() else {
            return Ok(vec![]);
        };
        let pathfinder = state.get_card(card_id);
        let site = state.get_card(&site_id);
        let valid_play_zones = site.get_valid_play_zones(state, player_id, card_id)?;
        let zones = pathfinder
            .get_zone()
            .get_adjacent_locations(state)
            .into_iter()
            .filter(|zone| valid_play_zones.contains(zone))
            .collect::<Vec<Zone>>();
        if zones.is_empty() {
            return Ok(vec![]);
        }

        let locations = crate::game::zones_to_locations(&zones);
        let zone = pick_location(
            player_id,
            &locations,
            state,
            false,
            "Pathfinder: Pick an adjacent location for your top site",
        )
        .await?;

        Ok(vec![
            Effect::RearrangeDeck {
                spells: deck.spells,
                sites: deck.sites,
            },
            Effect::PlayCard {
                player_id: *player_id,
                card_id: site_id,
                location: zone.clone(),
                spellcaster: *card_id,
            },
            Effect::MoveCard {
                player_id: *player_id,
                card_id: *card_id,
                from: (pathfinder.get_zone().clone())
                    .into_location()
                    .expect("MoveCard source must be a location"),
                to: LocationQuery::from_location(
                    (zone).with_region(pathfinder.get_region(state).clone()),
                ),
                tap: false,
                through_path: None,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct Pathfinder {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Pathfinder {
    pub const NAME: &'static str = "Pathfinder";
    pub const DESCRIPTION: &'static str = "Your atlas can’t contain duplicates. Draw no sites during setup.\r \r Tap → If able, play the topmost site of your atlas to an adjacent location and move there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Pathfinder {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_avatar(&self) -> Option<&dyn Avatar> {
        Some(self)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(PathfinderSitewalk)])
    }
}

impl Avatar for Pathfinder {}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Pathfinder::NAME, |owner_id: PlayerId| {
    Box::new(Pathfinder::new(owner_id))
});
