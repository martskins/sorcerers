use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BrobdingnagBullfrog {
    unit_base: UnitBase,
    card_base: CardBase,
    swallowed_minion: Option<CardId>,
}

impl BrobdingnagBullfrog {
    pub const NAME: &'static str = "Brobdingnag Bullfrog";
    pub const DESCRIPTION: &'static str = "Genesis → Brobdingnag Bullfrog swallows another target minion here. He carries it disabled in his belly until he leaves the realm.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            swallowed_minion: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for BrobdingnagBullfrog {
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
        Ok(vec![
            Hook::genesis(self.get_id()),
            Hook::deathrite(self.get_id()),
        ])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let player_id = self.get_controller_id(state);
                let minion = CardQuery::new()
                    .minions()
                    .in_location(self.get_location().clone())
                    .id_not(*self.get_id())
                    .with_source_card(*self.get_id())
                    .with_prompt("Pick a minon to swallow")
                    .pick(&player_id, state, false)
                    .await?;
                let Some(minion) = minion else {
                    return Ok(vec![]);
                };

                Ok(vec![
                    Effect::SetCardData {
                        card_id: *self.get_id(),
                        data: std::sync::Arc::new(minion),
                    },
                    Effect::SetBearer {
                        card_id: minion,
                        bearer_id: Some(*self.get_id()),
                    },
                ])
            }
            DEATHRITE_HOOK_ID => {
                if let Some(swallowed_minion_id) = self.swallowed_minion {
                    return Ok(vec![Effect::SetBearer {
                        card_id: swallowed_minion_id,
                        bearer_id: None,
                    }]);
                }

                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(swallowed_minion_id) = data.downcast_ref::<uuid::Uuid>() {
            self.swallowed_minion = Some(*swallowed_minion_id);
        }

        Ok(())
    }

    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        if let Some(swallowed_minion) = self.swallowed_minion {
            Ok(vec![OngoingEffect::GrantStatus {
                status: CardStatus::Disabled,
                affected_cards: swallowed_minion.into(),
            }])
        } else {
            Ok(vec![])
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BrobdingnagBullfrog::NAME, |owner_id: PlayerId| {
        Box::new(BrobdingnagBullfrog::new(owner_id))
    });
