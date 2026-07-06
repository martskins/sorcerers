use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum GeomancerAbility {
    PlaySite,
    DrawSite,
    ReplaceRubble,
}

#[async_trait::async_trait]
impl ActivatedAbility for GeomancerAbility {
    fn get_name(&self) -> String {
        match self {
            GeomancerAbility::PlaySite => "Play Site".to_string(),
            GeomancerAbility::DrawSite => "Draw Site".to_string(),
            GeomancerAbility::ReplaceRubble => "Replace Rubble".to_string(),
        }
    }

    fn can_activate(
        &self,
        card_id: &CardId,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        match self {
            GeomancerAbility::PlaySite => Ok(true),
            GeomancerAbility::DrawSite => Ok(true),
            GeomancerAbility::ReplaceRubble => {
                let geomancer = state.get_card(card_id);
                Ok(!CardQuery::new()
                    .sites()
                    .named(Rubble::NAME.to_string())
                    .adjacent_to(geomancer.get_location())
                    .all(state)
                    .is_empty())
            }
        }
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            GeomancerAbility::PlaySite => {
                let cards = CardQuery::new()
                    .sites()
                    .in_zone(Zone::Hand)
                    .controlled_by(player_id)
                    .all(state);
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let picked_card = state.get_card(&picked_card_id);
                let avatar_id = state.get_player_avatar_id(player_id)?;
                // we pass avatar_id as the caster just to comply with the required parameters, but
                // no caster_id is actually needed here, since sites don't need one.
                let locations =
                    picked_card.get_valid_play_locations(state, player_id, &avatar_id)?;
                let prompt = "Pick a zone to play the site";
                let zone = LocationQuery::from_locations(locations)
                    .with_prompt(prompt)
                    .with_source_card(picked_card_id)
                    .pick(player_id, state)
                    .await?;
                geomancer_play_site_effects(
                    card_id,
                    player_id,
                    state,
                    picked_card_id,
                    zone.get_square().unwrap(),
                )
                .await
            }
            GeomancerAbility::DrawSite => Ok(AvatarAction::DrawSite
                .on_select(card_id, player_id, state)
                .await?),
            GeomancerAbility::ReplaceRubble => {
                let card = state.get_card(card_id);
                let cards = CardQuery::new()
                    .named(Rubble::NAME.to_string())
                    .adjacent_to(card.get_location())
                    .all(state);
                let picked_rubble = pick_card(
                    card.get_controller_id(state),
                    &cards,
                    state,
                    "Geomancer: Pick a rubble to replace with a site",
                )
                .await?;

                let rubble = state.get_card(&picked_rubble);
                let deck = state.decks.get(&card.get_controller_id(state)).unwrap();
                let site_id = deck.sites.last();

                match site_id {
                    Some(site_id) => Ok(vec![
                        Effect::BanishCard {
                            card_id: *rubble.get_id(),
                        },
                        Effect::SetCardZone {
                            card_id: *site_id,
                            zone: rubble.get_zone().clone(),
                        },
                    ]),
                    None => Ok(vec![]),
                }
            }
        }
    }
}

async fn geomancer_play_site_effects(
    geomancer_id: &PlayerId,
    player_id: &PlayerId,
    state: &State,
    site_id: PlayerId,
    square: u8,
) -> anyhow::Result<Vec<Effect>> {
    let mut effects = vec![Effect::PlayCard {
        player_id: *player_id,
        card_id: site_id,
        location: Location::Square(square, Region::Surface),
        spellcaster: *geomancer_id,
    }];

    let picked_site = state.get_card(&site_id);
    let is_earth_site = picked_site
        .get_resource_provider()
        .ok_or(anyhow::anyhow!("Not a site"))?
        .provided_affinity(state)?
        .element(&Element::Earth)
        > 0;
    if is_earth_site {
        let geomancer = state.get_card(geomancer_id);
        let locations = geomancer
            .get_location()
            .with_region(Region::Void)
            .get_adjacent(state)
            .into_iter()
            .filter(|location| location.get_square().unwrap_or_default() != square)
            .collect::<Vec<Location>>();
        if !locations.is_empty() {
            let picked_zone = LocationQuery::from_locations(locations)
                .with_prompt("Pick a void to fill with a rubble")
                .with_source_card(*geomancer_id)
                .pick(player_id, state)
                .await?;
            effects.push(Effect::SummonToken {
                player_id: geomancer.get_controller_id(state),
                token_type: TokenType::Rubble,
                location: picked_zone,
            });
        }
    }

    Ok(effects)
}

#[derive(Debug, Clone)]
pub struct Geomancer {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Geomancer {
    pub const NAME: &'static str = "Geomancer";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site. If you played an earth site, fill a void adjacent to you with Rubble.\r \r Tap → Replace an adjacent Rubble with the topmost site of your atlas.";

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
impl Card for Geomancer {
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
        Ok(vec![
            Box::new(GeomancerAbility::ReplaceRubble) as Box<dyn ActivatedAbility>
        ])
    }
}

#[async_trait::async_trait]
impl Avatar for Geomancer {
    fn get_play_site_ability(&self) -> Option<Box<dyn ActivatedAbility>> {
        Some(Box::new(GeomancerAbility::PlaySite))
    }

    fn get_draw_site_ability(&self) -> Option<Box<dyn ActivatedAbility>> {
        Some(Box::new(GeomancerAbility::DrawSite))
    }

    async fn play_site_at_square(
        &self,
        state: &State,
        player_id: &PlayerId,
        site_id: &CardId,
        square: u8,
    ) -> anyhow::Result<Vec<Effect>> {
        geomancer_play_site_effects(self.get_id(), player_id, state, *site_id, square).await
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Geomancer::NAME, |owner_id: PlayerId| {
    Box::new(Geomancer::new(owner_id))
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::SimpleVillage;

    fn setup_geomancer_state(
        site_in_hand: bool,
        adjacent_rubble: bool,
    ) -> (State, PlayerId, CardId) {
        let mut state = State::new_mock_state(vec![8]);
        let player_id = state.players[0].id;

        let mut geomancer = Geomancer::new(player_id);
        let geomancer_id = *geomancer.get_id();
        geomancer.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(geomancer));

        if site_in_hand {
            let mut site = SimpleVillage::new(player_id);
            site.set_zone(Zone::Hand);
            state.add_card(Box::new(site));
        }

        if adjacent_rubble {
            let mut rubble = Rubble::new(player_id);
            rubble.set_zone(Zone::Location(Location::Square(9, Region::Surface)));
            state.add_card(Box::new(rubble));
        }

        (state, player_id, geomancer_id)
    }

    fn ability_names(state: &State, geomancer_id: &CardId) -> anyhow::Result<Vec<String>> {
        Ok(state
            .get_card(geomancer_id)
            .get_activated_abilities(state)?
            .into_iter()
            .map(|ability| ability.get_name())
            .collect())
    }

    #[test]
    fn geomancer_gets_unit_actions_and_custom_avatar_site_actions() {
        let (state, _player_id, geomancer_id) = setup_geomancer_state(true, true);
        let names = ability_names(&state, &geomancer_id).unwrap();

        assert!(names.contains(&"Attack".to_string()));
        assert!(names.contains(&"Move".to_string()));
        assert!(names.contains(&"Play Site".to_string()));
        assert!(names.contains(&"Draw Site".to_string()));
        assert!(names.contains(&"Replace Rubble".to_string()));
    }

    #[test]
    fn geomancer_replace_rubble_activation_requires_adjacent_rubble() {
        let (state, player_id, geomancer_id) = setup_geomancer_state(true, false);
        let abilities = state
            .get_card(&geomancer_id)
            .get_activated_abilities(&state)
            .unwrap();
        let replace_rubble = abilities
            .iter()
            .find(|ability| ability.get_name() == GeomancerAbility::ReplaceRubble.get_name())
            .expect("Geomancer to have Replace Rubble");
        assert!(
            !replace_rubble
                .can_activate(&geomancer_id, &player_id, &state)
                .unwrap()
        );

        let (state, player_id, geomancer_id) = setup_geomancer_state(true, true);
        let abilities = state
            .get_card(&geomancer_id)
            .get_activated_abilities(&state)
            .unwrap();
        let replace_rubble = abilities
            .iter()
            .find(|ability| ability.get_name() == GeomancerAbility::ReplaceRubble.get_name())
            .expect("Geomancer to have Replace Rubble");
        assert!(
            replace_rubble
                .can_activate(&geomancer_id, &player_id, &state)
                .unwrap()
        );
    }

    #[test]
    fn geomancer_custom_abilities_have_tap_costs() {
        let (mut state, player_id, geomancer_id) = setup_geomancer_state(true, true);

        for ability in [
            GeomancerAbility::PlaySite,
            GeomancerAbility::DrawSite,
            GeomancerAbility::ReplaceRubble,
        ] {
            state.get_card_mut(&geomancer_id).set_tapped(false);
            let abilities = state
                .get_card(&geomancer_id)
                .get_activated_abilities(&state)
                .unwrap();
            let ability_name = ability.get_name();
            let actual_ability = abilities
                .iter()
                .find(|ability| ability.get_name() == ability_name)
                .unwrap_or_else(|| panic!("Geomancer to have {ability_name}"));
            let cost = actual_ability.get_cost(&geomancer_id, &state).unwrap();
            assert!(
                cost.can_afford(&state, player_id).unwrap(),
                "{ability_name} should be payable while Geomancer is untapped"
            );

            state.get_card_mut(&geomancer_id).set_tapped(true);
            assert!(
                !cost.can_afford(&state, player_id).unwrap(),
                "{ability_name} should not be payable while Geomancer is tapped"
            );
        }
    }

    #[tokio::test]
    async fn geomancer_draw_site_has_tap_cost_and_draws_site() {
        let (state, player_id, geomancer_id) = setup_geomancer_state(true, false);
        let abilities = state
            .get_card(&geomancer_id)
            .get_activated_abilities(&state)
            .unwrap();
        let draw_site = abilities
            .into_iter()
            .find(|ability| ability.get_name() == "Draw Site")
            .expect("Geomancer to have Draw Site");

        let cost = draw_site.get_cost(&geomancer_id, &state).unwrap();
        assert!(cost.can_afford(&state, player_id).unwrap());

        let effects = draw_site
            .on_select(&geomancer_id, &player_id, &state)
            .await
            .unwrap();
        assert!(matches!(
            effects.as_slice(),
            [Effect::DrawCard {
                player_id: effect_player_id,
                count: 1,
                kind: DrawKind::Site,
            }] if *effect_player_id == player_id
        ));
    }
}
