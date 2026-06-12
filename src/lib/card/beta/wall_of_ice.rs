use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WallOfIce {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl WallOfIce {
    pub const NAME: &'static str = "Wall of Ice";
    pub const DESCRIPTION: &'static str = "Conjure atop the border of a site you control.\r \r Units can't traverse Wall of Ice on the ground.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
        }
    }
}

impl Aura for WallOfIce {}

#[async_trait::async_trait]
impl Card for WallOfIce {
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

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    fn get_valid_play_locations(
        &self,
        state: &State,
        player_id: &PlayerId,
        _caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Location>> {
        Ok(border_zones_of_controlled_sites(state, player_id)
            .into_iter()
            .filter_map(Zone::into_location)
            .collect())
    }

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let affected_minions = CardQuery::new()
            .minions()
            .in_play()
            .all(state)
            .into_iter()
            .filter(|card_id| {
                let card = state.get_card(card_id);
                card.get_region(state) == &Region::Surface
                    && !card.has_ability(state, &Ability::Airborne)
            })
            .collect();

        Ok(vec![OngoingEffect::BlockMovementThrough {
            border: self.get_location().clone(),
            affected_cards: CardQuery::from_ids(affected_minions),
        }])
    }
}

fn border_zones_of_controlled_sites(state: &State, player_id: &PlayerId) -> Vec<Zone> {
    let controlled_sites: Vec<u8> = state
        .cards
        .values()
        .filter(|card| card.is_site())
        .filter(|card| card.get_controller_id(state) == *player_id)
        .filter_map(|card| match card.get_zone() {
            Zone::Location(Location::Square(square, Region::Surface)) => Some(*square),
            _ => None,
        })
        .collect();

    Location::all_intersections()
        .into_iter()
        .filter(|location| match location {
            Location::Intersection(squares, _) => squares
                .iter()
                .any(|square| controlled_sites.contains(square)),
            Location::Square(_, _) => false,
        })
        .map(Zone::from)
        .collect()
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (WallOfIce::NAME, |owner_id: PlayerId| {
    Box::new(WallOfIce::new(owner_id))
});
