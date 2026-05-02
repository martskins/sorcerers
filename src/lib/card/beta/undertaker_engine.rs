use crate::{
    card::{
        Ability, AbilityCounter, Card, CardBase, CardConstructor, Costs, Edition, MinionType,
        Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_cards},
    query::EffectQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct UndertakerEngine {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl UndertakerEngine {
    pub const NAME: &'static str = "Undertaker Engine";
    pub const DESCRIPTION: &'static str = "At the end of your turn, you may burrow and/or unburrow any combination of artifacts and minions at this site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                types: vec![MinionType::Automaton],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(7),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for UndertakerEngine {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id || !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let cards_here: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|card| card.get_zone() == self.get_zone())
            .filter(|card| card.is_minion() || card.is_artifact())
            .filter(|card| {
                matches!(
                    card.get_region(state),
                    Region::Surface | Region::Underground
                )
            })
            .map(|card| *card.get_id())
            .collect();
        if cards_here.is_empty() {
            return Ok(vec![]);
        }

        let selected = pick_cards(
            &controller_id,
            &cards_here,
            state,
            "Undertaker Engine: Pick any artifacts and minions to burrow or unburrow",
        )
        .await?;

        let mut effects = vec![];
        for card_id in selected {
            let card = state.get_card(&card_id);
            let from_region = card.get_region(state).clone();
            let to_region = if from_region == Region::Underground {
                Region::Surface
            } else {
                Region::Underground
            };

            if card.is_minion()
                && from_region == Region::Surface
                && !card.has_ability(state, &Ability::Burrowing)
            {
                effects.push(Effect::AddAbilityCounter {
                    card_id,
                    counter: AbilityCounter {
                        id: uuid::Uuid::new_v4(),
                        ability: Ability::Burrowing,
                        expires_on_effect: Some(EffectQuery::SetCardRegion {
                            card: CardQuery::from_id(card_id),
                            region: Some(Region::Surface),
                        }),
                    },
                });
            }

            effects.push(Effect::SetCardRegion {
                card_id,
                region: to_region,
                tap: false,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (UndertakerEngine::NAME, |owner_id: PlayerId| {
        Box::new(UndertakerEngine::new(owner_id))
    });
