use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct HighlandPrincess {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HighlandPrincess {
    pub const NAME: &'static str = "Highland Princess";
    pub const DESCRIPTION: &'static str = "Genesis → Search your spellbook for an artifact that costs ① or less, reveal it, and put it into your hand. Shuffle.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "AA"),
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
impl Card for HighlandPrincess {
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
                let Some(chosen) = CardQuery::new()
                    .artifacts()
                    .in_zone(Zone::Spellbook)
                    .owned_by(&controller_id)
                    .with_prompt("Choose an artifact to reveal and put into your hand")
                    .with_source_card(*self.get_id())
                    .pick(&controller_id, state)
                    .await?
                else {
                    return Ok(vec![Effect::ShuffleDeck {
                        player_id: controller_id,
                    }]);
                };

                Ok(vec![
                    Effect::SetCardZone {
                        card_id: chosen,
                        zone: Zone::Hand,
                    },
                    Effect::ShuffleDeck {
                        player_id: controller_id,
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HighlandPrincess::NAME, |owner_id: PlayerId| {
        Box::new(HighlandPrincess::new(owner_id))
    });
