use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    query::ZoneQuery,
    state::{CardMatcher, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct BrobdingnagBullfrog {
    unit_base: UnitBase,
    card_base: CardBase,
    swallowed_minion: Option<uuid::Uuid>,
}

impl BrobdingnagBullfrog {
    pub const NAME: &'static str = "Brobdingnag Bullfrog";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "WW"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
            swallowed_minion: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for BrobdingnagBullfrog {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let minions = self.get_zone().get_minion_ids(state, None);
        let picked_card = pick_card(
            self.get_controller_id(state),
            &minions,
            state,
            "Brobdingnag Bullfrog: Pick a minon to swallow",
        )
        .await?;

        Ok(vec![Effect::SetCardData {
            card_id: self.get_id().clone(),
            data: Box::new(picked_card),
        }])
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(swallowed_minion_id) = data.downcast_ref::<uuid::Uuid>() {
            self.swallowed_minion = Some(swallowed_minion_id.clone());
        }

        Ok(())
    }

    async fn on_move(&self, state: &State, path: &[Zone]) -> anyhow::Result<Vec<Effect>> {
        if let Some(minion) = self.swallowed_minion {
            if let Some(zone) = path.last() {
                if zone.is_in_play() {
                    return Ok(vec![Effect::MoveCard {
                        card_id: minion.clone(),
                        to: ZoneQuery::from_zone(zone.clone()),
                        player_id: self.get_controller_id(state),
                        from: path.first().expect("Path should have at least one element").clone(),
                        tap: false,
                        region: self.get_region(state).clone(),
                        through_path: None,
                    }]);
                }
            }
        }

        Ok(vec![])
    }

    async fn get_continuous_effects(&self, _state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        Ok(vec![ContinuousEffect::GrantAbility {
            ability: Ability::Disabled,
            affected_cards: CardMatcher::from_id(self.swallowed_minion.unwrap_or(uuid::Uuid::nil()).clone()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (BrobdingnagBullfrog::NAME, |owner_id: PlayerId| {
        Box::new(BrobdingnagBullfrog::new(owner_id))
    });
