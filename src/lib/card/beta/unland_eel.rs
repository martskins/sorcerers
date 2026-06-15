use crate::prelude::*;

const SUBMERGE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct UnlandEel {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl UnlandEel {
    pub const NAME: &'static str = "Unland Eel";
    pub const DESCRIPTION: &'static str = "Submerge
Whenever Unland Eel submerges, it may drag another minion here down with it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for UnlandEel {
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
            id: SUBMERGE_HOOK,
            trigger: EffectQuery::SetCardRegion {
                card: self.get_id().into(),
                destination: Some(Region::Underwater),
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
            SUBMERGE_HOOK => {
                let controller_id = self.get_controller_id(state);
                let other_minions = CardQuery::new()
                    .minions()
                    .in_zone(self.get_zone())
                    .id_not(*self.get_id())
                    .all(state);
                if other_minions.is_empty() {
                    return Ok(vec![]);
                }

                if !yes_or_no(
                    &controller_id,
                    state,
                    "Drag another minion down with it?",
                    *self.get_id(),
                )
                .await?
                {
                    return Ok(vec![]);
                }

                let target_id = pick_card(
                    &controller_id,
                    &other_minions,
                    state,
                    "Unland Eel: Pick another minion here to drag down",
                )
                .await?;
                let target = state.get_card(&target_id);

                let mut effects = vec![];
                if target.get_region(state) != &Region::Underwater
                    && !target.has_ability(state, &Ability::Submerge)
                {
                    effects.push(Effect::AddAbilityCounter {
                        card_id: target_id,
                        counter: AbilityCounter {
                            id: uuid::Uuid::new_v4(),
                            ability: Ability::Submerge,
                            expires_on_effect: Some(EffectQuery::SetCardRegion {
                                card: CardQuery::from_id(target_id),
                                destination: Some(Region::Surface),
                            }),
                        },
                    });
                }
                effects.push(Effect::SetCardRegion {
                    card_id: target_id,
                    destination: Region::Underwater,
                    tap: false,
                });
                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (UnlandEel::NAME, |owner_id: PlayerId| {
    Box::new(UnlandEel::new(owner_id))
});
