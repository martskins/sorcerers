use crate::prelude::*;

#[derive(Debug, Clone)]
struct SettleAction;

fn adjacent_void_or_rubble(card_id: &CardId, state: &State) -> Vec<Location> {
    let card = state.get_card(card_id);
    let mut locations = card
        .get_location()
        .get_adjacent_squares()
        .into_iter()
        .filter_map(|location| {
            location
                .square()
                .map(|square| Location::Square(square, Region::Surface))
        })
        .filter(|location| match location.get_site(state) {
            None => true,
            Some(site) => site.get_name() == Rubble::NAME,
        })
        .collect::<Vec<_>>();
    locations.sort();
    locations.dedup();
    locations
}

fn valid_settle_locations(
    card_id: &CardId,
    site_id: &CardId,
    _player_id: &PlayerId,
    state: &State,
) -> anyhow::Result<Vec<Location>> {
    let site = state.get_card(site_id);
    if !site.base_playable_regions(state).contains(&Region::Surface) {
        return Ok(vec![]);
    }

    Ok(adjacent_void_or_rubble(card_id, state))
}

#[async_trait::async_trait]
impl ActivatedAbility for SettleAction {
    fn get_name(&self) -> String {
        "Tap → Reveal and play topmost site to adjacent void or Rubble; move there and lose this ability".to_string()
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
        Ok(!valid_settle_locations(card_id, site_id, player_id, state)?.is_empty())
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let site_id = match state.decks.get(player_id).and_then(|d| d.sites.last()) {
            Some(id) => *id,
            None => return Ok(vec![]),
        };
        let valid_locations = valid_settle_locations(card_id, &site_id, player_id, state)?;
        if valid_locations.is_empty() {
            return Ok(vec![]);
        }

        let location = LocationQuery::from_locations(valid_locations)
            .with_source_card(*card_id)
            .with_prompt("Pick an adjacent or Rubble zone to settle")
            .pick(player_id, state)
            .await?;
        Ok(vec![
            // Move the settlers to their new home.
            Effect::MoveCard {
                player_id: *player_id,
                card_id: *card_id,
                from: state.get_card(card_id).get_location().clone(),
                to: location.clone().into(),
                tap: false,
                through_path: None,
            },
            Effect::SummonCards {
                summoned_cards: vec![SummonCard {
                    player_id: *player_id,
                    card_id: site_id,
                    from_zone: Zone::Atlasbook,
                    to_location: location,
                }],
            },
            Effect::SetCardData {
                card_id: *card_id,
                data: std::sync::Arc::new(false),
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct FrontierSettlers {
    unit_base: UnitBase,
    card_base: CardBase,
    has_ability: bool,
}

impl FrontierSettlers {
    pub const NAME: &'static str = "Frontier Settlers";
    pub const DESCRIPTION: &'static str = "Tap → Reveal and play your topmost site to an adjacent void or Rubble. Frontier Settlers move there and lose this ability.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "EE"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            has_ability: true,
        }
    }
}

#[async_trait::async_trait]
impl Card for FrontierSettlers {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(has_ability) = data.downcast_ref::<bool>() {
            self.has_ability = *has_ability;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for Frontier Settlers"))
        }
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if !self.has_ability {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(SettleAction)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (FrontierSettlers::NAME, |owner_id: PlayerId| {
        Box::new(FrontierSettlers::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::SimpleVillage;

    #[test]
    fn frontier_settlers_only_offers_surface_locations_for_top_site() {
        let mut state = State::new_mock_state([3]);
        let player_id = state.players[0].id;

        let mut settlers = FrontierSettlers::new(player_id);
        let settlers_id = *settlers.get_id();
        settlers.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
        state.add_card(Box::new(settlers));

        let site = SimpleVillage::new(player_id);
        let site_id = *site.get_id();
        state.add_card(Box::new(site));
        state
            .get_player_deck_mut(&player_id)
            .expect("player deck should exist")
            .sites
            .push(site_id);

        let locations = valid_settle_locations(&settlers_id, &site_id, &player_id, &state)
            .expect("settle locations should resolve");

        assert!(!locations.is_empty());
        assert!(
            locations
                .iter()
                .all(|location| location.region() == &Region::Surface)
        );
        assert!(!locations.contains(&Location::Square(2, Region::Underground)));
        assert!(!locations.contains(&Location::Square(2, Region::Underwater)));
        assert!(!locations.contains(&Location::Square(2, Region::Void)));
    }
}
