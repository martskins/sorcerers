use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ColickyDragonettes {
    unit_base: UnitBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl ColickyDragonettes {
    pub const NAME: &'static str = "Colicky Dragonettes";
    pub const DESCRIPTION: &'static str =
        "At the end of your turn, Colicky Dragonettes shoot a projectile. It deals 1 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Dragon],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ColickyDragonettes {
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
            id: TURN_END_HOOK,
            trigger: EffectQuery::TurnEnd { player_id: None },
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
                let is_current_player = &state.current_player() == self.get_owner_id();
                if !is_current_player {
                    return Ok(vec![]);
                }

                let prompt = "Choose a direction to shoot a projectile";
                let direction = pick_direction_source(
                    self.get_owner_id(),
                    &CARDINAL_DIRECTIONS,
                    state,
                    prompt,
                    Some(*self.get_id()),
                )
                .await?;
                Ok(vec![Effect::ShootProjectile {
                    id: uuid::Uuid::new_v4(),
                    range: None,
                    player_id: *self.get_owner_id(),
                    shooter: *self.get_id(),
                    origin: self.get_location().clone(),
                    direction,
                    damage: 1,
                    ranged_strike: false,
                    piercing: false,
                    splash_damage: None,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ColickyDragonettes::NAME, |owner_id: PlayerId| {
        Box::new(ColickyDragonettes::new(owner_id))
    });
