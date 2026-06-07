use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct HighlandFalconer {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HighlandFalconer {
    pub const NAME: &'static str = "Highland Falconer";
    pub const DESCRIPTION: &'static str = "Genesis → You may search your hand and spellbook for a Beast with Airborne and mana cost ② or less and summon it here. Shuffle if needed.";

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
                costs: Costs::basic(2, "A"),
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
impl Card for HighlandFalconer {
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

                let targets = CardQuery::new()
                    .minions()
                    .in_zones(&[Zone::Hand, Zone::Spellbook])
                    .controlled_by(&controller_id)
                    .with_abilities(vec![Ability::Airborne])
                    .minion_type(&crate::card::MinionType::Beast)
                    .mana_cost_lte(2)
                    .all(state);

                if targets.is_empty() {
                    return Ok(vec![]);
                }

                let chosen = pick_card(
                    &controller_id,
                    &targets,
                    state,
                    "Highland Falconer: Summon a Beast with Airborne and mana cost 2 or less",
                )
                .await?;
                let from_zone = state.get_card(&chosen).get_zone().clone();
                let mut effects = vec![Effect::SummonCards {
                    summoned_cards: vec![SummonCard {
                        player_id: controller_id,
                        card_id: chosen,
                        from_zone: Zone::Spellbook,
                        to_location: self
                            .get_zone()
                            .clone()
                            .into_location()
                            .expect("Highland Falconer must be in a location"),
                    }],
                }];
                if from_zone == Zone::Spellbook {
                    effects.push(Effect::ShuffleDeck {
                        player_id: controller_id,
                    });
                }
                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HighlandFalconer::NAME, |owner_id: PlayerId| {
        Box::new(HighlandFalconer::new(owner_id))
    });
