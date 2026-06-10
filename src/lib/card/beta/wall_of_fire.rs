use crate::{card::zones_cross_border, prelude::*};

const MOVE_THROUGH_HERE_HOOK: HookId = 1;

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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: MOVE_THROUGH_HERE_HOOK,
            trigger: EffectQuery::MoveCard {
                card: CardQuery::new().units(),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            MOVE_THROUGH_HERE_HOOK => {
                let Effect::MoveCard {
                    player_id,
                    card_id,
                    from,
                    through_path,
                    to,
                    ..
                } = effect
                else {
                    return Ok(vec![]);
                };

                let final_zone = match through_path {
                    Some(path) => path.last().cloned(),
                    None => Some(to.pick(player_id, state).await?.into()),
                };
                let Some(final_zone) = final_zone else {
                    return Ok(vec![]);
                };

                // Unit must pass through Wall of Fire, not merely stop on it.
                if final_zone == *self.get_zone() {
                    return Ok(vec![]);
                }

                let mut path = vec![from.clone().into()];
                match through_path {
                    Some(through_path) => path.extend(through_path.iter().cloned()),
                    None => path.push(final_zone),
                }

                if !path
                    .windows(2)
                    .any(|step| zones_cross_border(&step[0], &step[1], self.get_zone()))
                {
                    return Ok(vec![]);
                }

                Ok(std::iter::once(*card_id)
                    .chain(CardQuery::new().carried_by(card_id).units().all(state))
                    .map(|damaged_id| Effect::TakeDamage {
                        card_id: damaged_id,
                        from: *self.get_id(),
                        damage: Damage::basic(3),
                    })
                    .collect())
            }
            _ => Ok(vec![]),
        }
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

    Zone::all_intersections()
        .into_iter()
        .filter(|zone| match zone {
            Zone::Location(Location::Intersection(squares, _)) => squares
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
