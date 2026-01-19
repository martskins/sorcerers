use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, reveal_cards, take_action, yes_or_no},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct MotherNature {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl MotherNature {
    pub const NAME: &'static str = "Mother Nature";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Spirit],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(6, "WWW"),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MotherNature {
    fn get_name(&self) -> &str {
        Self::NAME
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

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let deck = state.get_player_deck(&controller_id)?;
        if let Some(top_card_id) = deck.peek_spell() {
            let player = state.get_player(&controller_id)?;
            let opponent_id = state.get_opponent_id(&controller_id)?;
            let cards = vec![top_card_id.clone()];
            reveal_cards(
                &opponent_id,
                &cards,
                state,
                &format!("Mother Nature: Seeing the top card of {}'s spellbook", player.name),
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
                    return Ok(vec![Effect::SummonCard {
                        player_id: controller_id.clone(),
                        card_id: top_card_id.clone(),
                        zone: self.get_zone().clone(),
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (MotherNature::NAME, |owner_id: PlayerId| {
    Box::new(MotherNature::new(owner_id))
});
