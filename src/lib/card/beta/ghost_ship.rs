use crate::prelude::*;

const SUMMON_SPIRIT_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct GhostShip {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GhostShip {
    pub const NAME: &'static str = "Ghost Ship";
    pub const DESCRIPTION: &'static str = "Voidwalk\r \r Whenever Ghost Ship enters a site from the void, you may summon a Spirit from any cemetery to its location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Voidwalk],
                types: vec![MinionType::Spirit],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "W"),
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
impl Card for GhostShip {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: SUMMON_SPIRIT_HOOK,
            trigger: EffectQuery::EnterLocation {
                card: self.get_id().into(),
                location: Box::new(LocationQuery::any_site(None, None)),
                from: Some(Box::new(LocationQuery::any_void())),
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
            SUMMON_SPIRIT_HOOK => {
                let player_id = self.get_controller_id(state);
                let Some(target_spirit) = CardQuery::new()
                    .dead()
                    .minions()
                    .minion_type(&MinionType::Spirit)
                    .pick(&player_id, state)
                    .await?
                else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::SummonCards {
                    summoned_cards: vec![SummonCard {
                        player_id,
                        card_id: target_spirit,
                        from_zone: Zone::Cemetery,
                        to_location: self
                            .get_zone()
                            .clone()
                            .location()
                            .cloned()
                            .expect("Ghost Ship target must be a location"),
                    }],
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GhostShip::NAME, |owner_id: PlayerId| {
    Box::new(GhostShip::new(owner_id))
});
