use crate::{
    card::{Aura, AuraBase, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Thunderstorm {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Thunderstorm {
    pub const NAME: &'static str = "Thunderstorm";
    pub const DESCRIPTION: &'static str = "At the end of your turn, deal 3 damage to a random unit atop affected sites, then you may move Thunderstorm one step.\r \r Lasts 3 of your turns.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
        }
    }
}

impl Aura for Thunderstorm {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        let controller_id = self.get_controller_id(state);
        let turns_in_play = state
            .effect_log
            .iter()
            .skip_while(|e| match *e.effect {
                Effect::PlayCard { ref card_id, .. } if card_id == self.get_id() => false,
                _ => true,
            })
            .filter(|e| match *e.effect {
                Effect::EndTurn { ref player_id, .. } if player_id == &controller_id => true,
                _ => false,
            })
            .count();

        Ok(turns_in_play >= 3)
    }
}

#[async_trait::async_trait]
impl Card for Thunderstorm {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if state.current_player != self.get_controller_id(state) {
            return Ok(vec![]);
        }

        let zones = self.get_valid_move_zones(state)?;
        let affected_zones = self.get_affected_zones(state);
        // Add DealDamageToTarget after MoveCard, so that the damage effect is processed before the
        // move effect.
        let effects = vec![
            Effect::MoveCard {
                player_id: self.get_controller_id(state).clone(),
                card_id: self.get_id().clone(),
                from: self.get_zone().clone(),
                to: ZoneQuery::from_options(
                    zones,
                    Some("Pick a zone to move Thunderstorm to".to_string()),
                ),
                tap: false,
                region: self.get_region(state).clone(),
                through_path: None,
            },
            Effect::DealDamageToTarget {
                player_id: self.get_controller_id(state).clone(),
                query: CardQuery::new()
                    .randomised()
                    .count(1)
                    .units()
                    .in_zones(&affected_zones)
                    .id_not_in(vec![self.get_id().clone()]),
                from: self.get_id().clone(),
                damage: 3,
            },
        ];

        Ok(effects)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Thunderstorm::NAME, |owner_id: PlayerId| {
        Box::new(Thunderstorm::new(owner_id))
    });
