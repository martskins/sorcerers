use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_action, pick_card},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone, PartialEq)]
enum ClamorOfHarpiesAction {
    Strike,
    DoNotStrike,
}

#[async_trait::async_trait]
impl ActivatedAbility for ClamorOfHarpiesAction {
    fn get_name(&self) -> String {
        match self {
            ClamorOfHarpiesAction::Strike => "Strike".to_string(),
            ClamorOfHarpiesAction::DoNotStrike => "Do Not Strike".to_string(),
        }
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            ClamorOfHarpiesAction::Strike => {
                let target_card = state.get_card(card_id);
                Ok(vec![Effect::take_damage(
                    &target_card.get_id(),
                    card_id,
                    state.get_card(card_id).get_power(state)?.unwrap_or(0),
                )])
            }
            ClamorOfHarpiesAction::DoNotStrike => Ok(vec![]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClamorOfHarpies {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl ClamorOfHarpies {
    pub const NAME: &'static str = "Clamor of Harpies";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Monster],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "F"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ClamorOfHarpies {
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
        let valid_cards: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.can_be_targetted_by(state, &self.get_controller_id(state)))
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| {
                c.get_power(state).unwrap_or_default().unwrap_or(0)
                    < self.get_power(state).unwrap_or_default().unwrap_or(0)
            })
            .map(|c| c.get_id().clone())
            .collect();
        let prompt = "Clamor of Harpies: Pick a unit to bring here";
        let card_id = pick_card(self.get_controller_id(state), &valid_cards, state, prompt).await?;
        let card = state.get_card(&card_id);
        let activated_abilities: Vec<Box<dyn ActivatedAbility>> = vec![
            Box::new(ClamorOfHarpiesAction::Strike),
            Box::new(ClamorOfHarpiesAction::DoNotStrike),
        ];
        let prompt = "Clamor of Harpies: Strike selected unit?";
        let action = pick_action(self.get_controller_id(state), &activated_abilities, state, prompt).await?;
        let mut effects = vec![Effect::MoveCard {
            player_id: self.get_controller_id(state).clone(),
            card_id,
            from: card.get_zone().clone(),
            to: ZoneQuery::Specific {
                id: uuid::Uuid::new_v4(),
                zone: self.get_zone().clone(),
            },
            tap: false,
            region: self.card_base.region.clone(),
            through_path: None,
        }];
        effects.extend(
            action
                .on_select(card.get_id(), &self.get_controller_id(state), state)
                .await?,
        );
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (ClamorOfHarpies::NAME, |owner_id: PlayerId| {
    Box::new(ClamorOfHarpies::new(owner_id))
});
