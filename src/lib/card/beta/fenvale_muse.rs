use crate::prelude::*;

const TRIGGER_RIVER_GENESIS_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct FenvaleMuse {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl FenvaleMuse {
    pub const NAME: &'static str = "Fenvale Muse";
    pub const DESCRIPTION: &'static str = "Spellcaster\r \r Whenever Fenvale Muse casts a spell, you may trigger the Genesis of a nearby River.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 0,
                abilities: vec![Ability::Spellcaster(None)],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "W"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for FenvaleMuse {
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
            id: TRIGGER_RIVER_GENESIS_HOOK,
            trigger: EffectQuery::PlayCard {
                card: CardQuery::new().including_not_in_play(),
                spellcaster: Some(self.get_id().into()),
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
            TRIGGER_RIVER_GENESIS_HOOK => {
                let controller_id = self.get_controller_id(state);
                let rivers_query = CardQuery::new()
                    .sites()
                    .near_to(self.get_location())
                    .with_source_card(*self.get_id())
                    .with_prompt("Pick a nearby River to trigger")
                    .site_types(vec![SiteType::River]);
                if rivers_query.is_empty(state) {
                    return Ok(vec![]);
                }

                let want = yes_or_no(
                    &controller_id,
                    state,
                    "Trigger the Genesis of a nearby River?",
                    *self.get_id(),
                )
                .await?;
                if !want {
                    return Ok(vec![]);
                }

                let Some(river_id) = rivers_query.pick(&controller_id, state).await? else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::TriggerGenesis { card_id: river_id }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FenvaleMuse::NAME, |owner_id: PlayerId| {
    Box::new(FenvaleMuse::new(owner_id))
});
