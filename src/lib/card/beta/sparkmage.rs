use crate::{
    card::{AdditionalCost, AvatarBase, Card, CardBase, Cost, Costs, Edition, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, Element, PlayerId},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct DealDamageAction;

#[async_trait::async_trait]
impl ActivatedAbility for DealDamageAction {
    fn get_name(&self) -> String {
        "Tap to deal damage".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(CardQuery::from_id(
            card_id.clone(),
        ))))
    }

    fn can_activate(&self, card_id: &uuid::Uuid, _player_id: &PlayerId, state: &State) -> anyhow::Result<bool> {
        let zone = state.get_card(card_id).get_zone();
        Ok(!CardQuery::new()
            .count(1)
            .units()
            .randomised()
            .in_zone(&zone)
            .all(state)
            .is_empty())
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let sparkmage = state.get_card(card_id);
        let zone = ZoneQuery::new()
            .near(sparkmage.get_zone())
            .pick(player_id, state)
            .await?;
        let damage = state
            .effect_log
            .iter()
            .rev()
            .filter(|le| le.turn == state.turns)
            .fold(0, |acc, e| match *e.effect {
                Effect::PlayMagic {
                    player_id: pid,
                    card_id: cid,
                    ..
                } if &pid == player_id => {
                    let card = state.get_card(&cid);
                    acc + card
                        .get_costs(state)
                        .cloned()
                        .unwrap_or_default()
                        .thresholds_cost()
                        .element(&Element::Air)
                }
                Effect::PlayCard {
                    player_id: pid,
                    card_id: cid,
                    ..
                } if &pid == player_id => {
                    let card = state.get_card(&cid);
                    acc + card
                        .get_costs(state)
                        .cloned()
                        .unwrap_or_default()
                        .thresholds_cost()
                        .element(&Element::Air)
                }
                _ => acc,
            });
        Ok(vec![Effect::DealDamageToTarget {
            player_id: player_id.clone(),
            query: CardQuery::new().count(1).units().randomised().in_zone(&zone),
            from: card_id.clone(),
            damage: damage.into(),
        }])
    }
}

#[derive(Debug, Clone)]
pub struct Sparkmage {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl Sparkmage {
    pub const NAME: &'static str = "Sparkmage";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r Tap → Target nearby location. Deal damage to another random unit there equal to the sum of (A) on spells you've cast this turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase { ..Default::default() },
        }
    }
}

impl Card for Sparkmage {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(DealDamageAction)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Sparkmage::NAME, |owner_id: PlayerId| Box::new(Sparkmage::new(owner_id)));
