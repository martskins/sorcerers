use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WallOfFire {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl WallOfFire {
    pub const NAME: &'static str = "Wall of Fire";
    pub const DESCRIPTION: &'static str = "Conjure atop the border of a site you control.\r \r Whenever a unit passes through Wall of Fire, it takes 3 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
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

impl Aura for WallOfFire {}

#[async_trait::async_trait]
impl Card for WallOfFire {
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

    fn get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
        _caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Zone>> {
        Ok(border_zones_of_controlled_sites(state, player_id))
    }

    async fn on_effect(&self, state: &State, effect: &Effect) -> anyhow::Result<Vec<Effect>> {
        let Effect::MoveCard {
            player_id,
            card_id,
            from,
            to,
            through_path,
            ..
        } = effect
        else {
            return Ok(vec![]);
        };

        if !state.get_card(card_id).is_unit() {
            return Ok(vec![]);
        }

        let mut path = vec![from.clone()];
        match through_path {
            Some(steps) => path.extend(steps.clone()),
            None => path.push(to.pick(player_id, state).await?),
        }

        if path
            .windows(2)
            .any(|zones| zones_cross_border(&zones[0], &zones[1], self.get_zone()))
        {
            return Ok(vec![Effect::TakeDamage {
                card_id: *card_id,
                from: *self.get_id(),
                damage: Damage::basic(3),
            }]);
        }

        Ok(vec![])
    }
}

fn zones_cross_border(from: &Zone, to: &Zone, border: &Zone) -> bool {
    let (Some(from_square), Some(to_square)) = (from.get_square(), to.get_square()) else {
        return false;
    };

    match border {
        Zone::Intersection(squares, _) => {
            squares.contains(&from_square) && squares.contains(&to_square)
        }
        _ => false,
    }
}

fn border_zones_of_controlled_sites(state: &State, player_id: &PlayerId) -> Vec<Zone> {
    let controlled_sites: Vec<u8> = state
        .cards
        .values()
        .filter(|card| card.is_site())
        .filter(|card| card.get_controller_id(state) == *player_id)
        .filter_map(|card| match card.get_zone() {
            Zone::Realm(square, Region::Surface) => Some(*square),
            _ => None,
        })
        .collect();

    Zone::all_intersections()
        .into_iter()
        .filter(|zone| match zone {
            Zone::Intersection(squares, _) => squares
                .iter()
                .any(|square| controlled_sites.contains(square)),
            _ => false,
        })
        .collect()
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (WallOfFire::NAME, |owner_id: PlayerId| {
    Box::new(WallOfFire::new(owner_id))
});
