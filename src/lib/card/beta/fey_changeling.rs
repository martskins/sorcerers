use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct FeyChangeling {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl FeyChangeling {
    pub const NAME: &'static str = "Fey Changeling";
    pub const DESCRIPTION: &'static str = "May be summoned to any site.\r \r Genesis → You may return a minion here to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![],
                types: vec![MinionType::Fairy],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
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
impl Card for FeyChangeling {
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

    fn get_valid_play_locations(
        &self,
        state: &State,
        _player_id: &PlayerId,
        _caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Location>> {
        Ok(CardQuery::new()
            .sites()
            .in_play()
            .all(state)
            .into_iter()
            .map(|cid| state.get_card(&cid).get_location().clone())
            .collect())
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let controller_id = self.get_controller_id(state);
                let minion_query = CardQuery::new()
                    .units()
                    .in_location(self.get_location().clone())
                    .with_source_card(*self.get_id())
                    .with_prompt("Pick a minion to bounce")
                    .id_not(*self.get_id());
                if !minion_query.has_targets(state) {
                    return Ok(vec![]);
                }

                let want = yes_or_no(
                    &controller_id,
                    state,
                    "Return a minion here to its owner's hand?",
                    *self.get_id(),
                )
                .await?;
                if !want {
                    return Ok(vec![]);
                }

                let Some(target_id) = minion_query.pick(&controller_id, state).await? else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::SetCardZone {
                    card_id: target_id,
                    zone: Zone::Hand,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (FeyChangeling::NAME, |owner_id: PlayerId| {
        Box::new(FeyChangeling::new(owner_id))
    });
