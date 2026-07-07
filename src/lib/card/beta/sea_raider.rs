use crate::prelude::*;

const KILL_MINION_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct SeaRaider {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SeaRaider {
    pub const NAME: &'static str = "Sea Raider";
    pub const DESCRIPTION: &'static str = "Whenever Sea Raider attacks and kills an enemy, its controller discards their topmost spell. You may cast that spell once this turn, ignoring threshold.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
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
        }
    }
}

#[async_trait::async_trait]
impl Card for SeaRaider {
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

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let player_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&player_id)?;
        Ok(vec![Hook {
            id: KILL_MINION_HOOK,
            trigger: EffectQuery::UnitKilled {
                unit: Box::new(CardQuery::new().minions().controlled_by(&opponent_id)),
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
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            KILL_MINION_HOOK => {
                let Effect::KillMinion { card_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let player_id = self.get_controller_id(state);
                let opponent_id = state.get_card(card_id).get_controller_id(state);
                let Some(&spell_id) = state.get_player_deck(&opponent_id)?.peek_spell() else {
                    return Ok(vec![]);
                };

                let expires_on_effect = EffectQuery::OneOf(vec![
                    EffectQuery::TurnEnd {
                        player_id: Some(player_id),
                    },
                    EffectQuery::PlayCard {
                        card: Box::new(CardQuery::from_id(spell_id)),
                        spellcaster: None,
                    },
                ]);

                Ok(vec![
                    Effect::SetCardZone {
                        card_id: spell_id,
                        zone: Zone::Cemetery,
                    },
                    Effect::AddTemporaryEffect {
                        effect: TemporaryEffect::MakePlayable {
                            affected_cards: Box::new(
                                CardQuery::from_id(spell_id).including_not_in_play(),
                            ),
                            expires_on_effect: expires_on_effect.clone(),
                            by_player: player_id,
                        },
                    },
                    Effect::AddTemporaryEffect {
                        effect: TemporaryEffect::IgnoreCostThresholds {
                            affected_cards: Box::new(
                                CardQuery::from_id(spell_id).including_not_in_play(),
                            ),
                            expires_on_effect,
                            for_player: player_id,
                        },
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SeaRaider::NAME, |owner_id: PlayerId| {
    Box::new(SeaRaider::new(owner_id))
});
