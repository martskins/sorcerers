use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MotherNature {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MotherNature {
    pub const NAME: &'static str = "Mother Nature";
    pub const DESCRIPTION: &'static str = "At the start of your turn, reveal your topmost spell. If it's a minion, you may summon it here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Spirit],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "WWW"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

const TURN_START_HOOK: HookId = 1;

#[async_trait::async_trait]
impl Card for MotherNature {
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
            id: TURN_START_HOOK,
            trigger: EffectQuery::TurnStart { player_id: None },
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
            TURN_START_HOOK => {
                let controller_id = self.get_controller_id(state);
                if state.current_player() != controller_id {
                    return Ok(vec![]);
                }
                let deck = state.get_player_deck(&controller_id)?;
                if let Some(top_card_id) = deck.peek_spell() {
                    let player = state.get_player(&controller_id)?;
                    let opponent_id = state.get_opponent_id(&controller_id)?;
                    let cards = vec![*top_card_id];
                    reveal_cards(
                        &opponent_id,
                        &cards,
                        state,
                        &format!(
                            "Mother Nature: Seeing the top card of {}'s spellbook",
                            player.name
                        ),
                    )
                    .await?;

                    let card = state.get_card(top_card_id);
                    if card.is_minion() {
                        let summon = take_action(
                            &player.id,
                            &cards,
                            state,
                            "Mother Nature: Seeing the top card of your spellbook",
                            "Mother Nature: Summon minion here?",
                        )
                        .await?;

                        if summon {
                            return Ok(vec![Effect::SummonCards {
                                summoned_cards: vec![SummonCard {
                                    player_id: controller_id,
                                    card_id: *top_card_id,
                                    from_zone: Zone::Spellbook,
                                    to_location: self
                                        .get_zone()
                                        .clone()
                                        .location().cloned()
                                        .expect("Mother Nature must be in a location"),
                                }],
                            }]);
                        }
                    } else {
                        reveal_cards(
                            &player.id,
                            &cards,
                            state,
                            "Mother Nature: Seeing the top card of your spellbook",
                        )
                        .await?;
                    }
                }

                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MotherNature::NAME, |owner_id: PlayerId| {
    Box::new(MotherNature::new(owner_id))
});
