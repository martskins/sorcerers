use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ConquerorWorm {
    unit_base: UnitBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl ConquerorWorm {
    pub const NAME: &'static str = "Conqueror Worm";
    pub const DESCRIPTION: &'static str = "At the end of your turn, if no enemy units occupy this site, permanently gain control of it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 8,
                toughness: 8,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(8, "EE"),
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
impl Card for ConquerorWorm {
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

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TURN_END_HOOK,
            trigger: EffectQuery::TurnEnd {
                player_id: Some(self.get_controller_id(state)),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TURN_END_HOOK => {
                let controller_id = self.get_controller_id(state);
                let location = self.get_location();

                // Get the site card at this zone.
                let Some(site) = location.get_site(state) else {
                    return Ok(vec![]);
                };

                // Already controlled by us?
                if site.get_controller_id(state) == controller_id {
                    return Ok(vec![]);
                }

                let enemy_units = CardQuery::new()
                    .units()
                    .not_controlled_by(&controller_id)
                    .in_location(location.clone())
                    .all(state);
                if !enemy_units.is_empty() {
                    return Ok(vec![]);
                }

                Ok(vec![Effect::SetController {
                    card_id: *site.get_id(),
                    player_id: controller_id,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ConquerorWorm::NAME, |owner_id: PlayerId| {
        Box::new(ConquerorWorm::new(owner_id))
    });
