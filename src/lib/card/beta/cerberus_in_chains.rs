use crate::prelude::*;

const ON_AVATAR_MOVE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct CerberusInChains {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl CerberusInChains {
    pub const NAME: &'static str = "Cerberus in Chains";
    pub const DESCRIPTION: &'static str = "Must be summoned to your location.\r \r Cerberus in Chains automatically follows you and can't move itself away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Immobile],
                types: vec![MinionType::Demon],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CerberusInChains {
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

    /// Cerberus must be summoned to the owner's avatar zone.
    fn get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
        _caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Zone>> {
        let avatar_id = state.get_player_avatar_id(player_id)?;
        let avatar_zone = state.get_card(&avatar_id).get_zone().clone();
        Ok(vec![avatar_zone])
    }

    async fn get_valid_move_locations(&self, _state: &State) -> anyhow::Result<Vec<Location>> {
        Ok(vec![self.get_location().clone()]) // Cerberus can't move itself.
    }

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let player_id = self.get_controller_id(state);
        let avatar_id = state.get_player_avatar_id(&player_id)?;
        Ok(vec![Hook {
            id: ON_AVATAR_MOVE_HOOK,
            trigger: EffectQuery::MoveCard {
                card: avatar_id.into(),
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
            ON_AVATAR_MOVE_HOOK => {
                let player_id = self.get_controller_id(state);
                let avatar_id = state.get_player_avatar_id(&player_id)?;
                let avatar = state.get_card(&avatar_id);
                let new_zone = avatar.get_zone().clone();

                Ok(vec![Effect::MoveCard {
                    player_id,
                    card_id: *self.get_id(),
                    from: self
                        .get_zone()
                        .clone()
                        .into_location()
                        .expect("Cerberus must be in a location"),
                    to: LocationQuery::from_zone(new_zone),
                    tap: false,
                    through_path: None,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (CerberusInChains::NAME, |owner_id: PlayerId| {
        Box::new(CerberusInChains::new(owner_id))
    });
