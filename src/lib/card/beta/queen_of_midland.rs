use crate::prelude::*;

const OPPONENT_DRAW_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct QueenOfMidland {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl QueenOfMidland {
    pub const NAME: &'static str = "Queen of Midland";
    pub const DESCRIPTION: &'static str =
        "After an opponent draws a card, if they have more cards than you, you may draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "EE"),
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
impl Card for QueenOfMidland {
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
    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let player_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&player_id)?;
        Ok(vec![Hook {
            id: OPPONENT_DRAW_HOOK,
            trigger: EffectQuery::DrawCard {
                player_id: Some(opponent_id),
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
            OPPONENT_DRAW_HOOK => {
                let player_id = self.get_controller_id(state);
                let opponent_id = state.get_opponent_id(&player_id)?;
                let my_hand = CardQuery::new()
                    .in_zone(&Zone::Hand)
                    .controlled_by(&player_id)
                    .all(state)
                    .len();
                let opp_hand = CardQuery::new()
                    .in_zone(&Zone::Hand)
                    .controlled_by(&opponent_id)
                    .all(state)
                    .len();
                if opp_hand <= my_hand {
                    return Ok(vec![]);
                }
                let draw =
                    yes_or_no_source(&player_id, state, "Draw a card?", Some(*self.get_id()))
                        .await?;
                if draw {
                    Ok(vec![Effect::DrawCard {
                        player_id,
                        count: 1,
                        kind: DrawKind::Choice,
                    }])
                } else {
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (QueenOfMidland::NAME, |owner_id: PlayerId| {
        Box::new(QueenOfMidland::new(owner_id))
    });
