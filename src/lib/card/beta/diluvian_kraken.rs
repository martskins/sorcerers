use crate::{
    card::{Ability, AdditionalCost, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds},
    query::{CardQuery, ZoneQuery},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
struct TapToStrikeNearbyMinions;

#[async_trait::async_trait]
impl ActivatedAbility for TapToStrikeNearbyMinions {
    fn get_name(&self) -> String {
        "Tap to Strike Nearby Minions".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let kraken = state.get_card(card_id);
        let units_nearby = CardMatcher::units_near(kraken.get_zone())
            .not_in_ids(vec![kraken.get_id().clone()])
            .resolve_ids(state);
        let mut effects: Vec<Effect> = units_nearby
            .into_iter()
            .map(|unit_id| Effect::TakeDamage {
                card_id: unit_id.clone(),
                from: kraken.get_id().clone(),
                damage: kraken.get_power(state).unwrap().unwrap(),
            })
            .collect();

        effects.push(Effect::MoveCard {
            player_id: player_id.clone(),
            card_id: card_id.clone(),
            from: kraken.get_zone().clone(),
            to: ZoneQuery::from_zone(kraken.get_zone().clone()),
            tap: false,
            region: Region::Surface,
            through_path: None,
        });

        Ok(effects)
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost {
            mana: 0,
            thresholds: Thresholds::new(),
            additional: vec![AdditionalCost::Surface {
                card: CardQuery::from_id(card_id.clone()),
            }],
        })
    }
}

#[derive(Debug, Clone)]
pub struct DiluvianKraken {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl DiluvianKraken {
    pub const NAME: &'static str = "Diluvian Kraken";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 8,
                toughness: 8,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Monster],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(8, "WWW"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for DiluvianKraken {
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

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(TapToStrikeNearbyMinions)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (DiluvianKraken::NAME, |owner_id: PlayerId| {
    Box::new(DiluvianKraken::new(owner_id))
});
