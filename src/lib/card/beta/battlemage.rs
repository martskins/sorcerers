use crate::prelude::*;

const KILL_ENEMY_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct Battlemage {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Battlemage {
    pub const NAME: &'static str = "Battlemage";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r Whenever Battlemage attacks and kills an enemy, you may draw a spell.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
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
impl Card for Battlemage {
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

    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let player_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&player_id)?;
        Ok(vec![Hook {
            id: KILL_ENEMY_HOOK,
            trigger: EffectQuery::UnitKilled {
                unit: CardQuery::new().units().controlled_by(&opponent_id),
                killer: Some(self.get_id().into()),
                from_attack: Some(true),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            KILL_ENEMY_HOOK => {
                let controller_id = self.get_controller_id(state);
                let draw =
                    yes_or_no_source(controller_id, state, "Draw a spell?", Some(*self.get_id()))
                        .await?;
                if !draw {
                    return Ok(vec![]);
                }

                Ok(vec![Effect::DrawCard {
                    player_id: controller_id,
                    count: 1,
                    kind: DrawKind::Spell,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

impl Avatar for Battlemage {}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Battlemage::NAME, |owner_id: PlayerId| {
    Box::new(Battlemage::new(owner_id))
});
