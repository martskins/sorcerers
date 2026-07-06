use crate::prelude::*;

const TRACK_VISITED_SITE_HOOK: HookId = 1;
const TURN_END_HOOK: HookId = 2;

#[derive(Debug, Clone)]
pub struct Wildfire {
    aura_base: AuraBase,
    card_base: CardBase,
    sites_visited: Vec<Location>,
}

impl Wildfire {
    pub const NAME: &'static str = "Wildfire";
    pub const DESCRIPTION: &'static str = "Conjure Wildfire atop a single site nearby.\r \r At the end of each turn, each unit here takes 3 damage, then move Wildfire to an adjacent location it hasn't visited before. If none remain, dispel Wildfire.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "F"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                needs_explicit_spellcaster: true,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
            sites_visited: vec![],
        }
    }

    fn unvisited_adjacent_locations(&self, state: &State) -> Vec<Location> {
        self.get_location()
            .get_adjacent(state)
            .into_iter()
            .filter(|location| !self.sites_visited.contains(location))
            .collect()
    }
}

impl Aura for Wildfire {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        Ok(self.unvisited_adjacent_locations(state).is_empty())
    }
}

#[async_trait::async_trait]
impl Card for Wildfire {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(sites_visited) = data.downcast_ref::<Vec<Location>>() {
            self.sites_visited = sites_visited.clone();
        }

        Ok(())
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![
            Hook {
                id: TRACK_VISITED_SITE_HOOK,
                trigger: EffectQuery::EnterLocation {
                    card: self.get_id().into(),
                    location: LocationQuery::new(),
                    from: None,
                },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
            Hook {
                id: TURN_END_HOOK,
                trigger: EffectQuery::TurnEnd { player_id: None },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
        ])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TRACK_VISITED_SITE_HOOK => {
                let mut sites_visited = self.sites_visited.clone();
                sites_visited.push(self.get_location().clone());
                Ok(vec![Effect::SetCardData {
                    card_id: *self.get_id(),
                    data: std::sync::Arc::new(sites_visited),
                }])
            }
            TURN_END_HOOK => {
                let zones = self.unvisited_adjacent_locations(state);
                if zones.is_empty() {
                    return Ok(vec![Effect::BuryCard {
                        card_id: *self.get_id(),
                    }]);
                }

                let mut effects = CardQuery::new()
                    .units()
                    .in_zone(self.get_zone())
                    .all(state)
                    .into_iter()
                    .map(|id| Effect::TakeDamage {
                        card_id: id,
                        from: *self.get_id(),
                        damage: Damage::basic(3),
                    })
                    .collect::<Vec<Effect>>();

                let prompt = "Pick a zone to move to";
                let picked_zone = LocationQuery::from_locations(zones)
                    .with_prompt(prompt)
                    .with_source_card(*self.get_id())
                    .pick(self.get_owner_id(), state)
                    .await?;
                effects.push(Effect::MoveCard {
                    player_id: *self.get_owner_id(),
                    card_id: *self.get_id(),
                    from: self
                        .get_zone()
                        .clone()
                        .location()
                        .cloned()
                        .expect("Wildfire must be in a location"),
                    to: LocationQuery::from_location(picked_zone),
                    tap: false,
                    through_path: None,
                });

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }

    fn get_valid_play_locations(
        &self,
        state: &State,
        _player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Location>> {
        Ok(state
            .get_card(caster_id)
            .get_location()
            .get_nearby_sites(state))
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Wildfire::NAME, |owner_id: PlayerId| {
    Box::new(Wildfire::new(owner_id))
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unvisited_adjacent_locations_only_uses_existing_locations() {
        let state = State::new_mock_state(vec![8, 13]);
        let player_id = state.players[0].id;
        let from_zone = Zone::Location(Location::Square(13, Region::Surface));
        let from_location = Location::Square(13, Region::Surface);

        let mut wildfire = Wildfire::new(player_id);
        wildfire.set_zone(from_zone.clone());
        wildfire.sites_visited = vec![from_location];

        assert_eq!(
            wildfire.unvisited_adjacent_locations(&state),
            vec![Location::Square(8, Region::Surface)]
        );
    }
}
