use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct TwistOfFate {
    card_base: CardBase,
}

impl TwistOfFate {
    pub const NAME: &'static str = "Twist of Fate";
    pub const DESCRIPTION: &'static str = "Exchange life totals with target opponent. (X) is the difference between your life totals.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::single(Cost::from_variable_mana("WWW")),
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
impl Card for TwistOfFate {
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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;
        let paid_mana = cost_paid
            .into_iter()
            .find_map(|cost| match cost {
                CostType::ManaCost(mana) => Some(mana),
                _ => None,
            })
            .unwrap_or_default();

        let controller_avatar_id = state.get_player_avatar_id(&controller_id)?;
        let opponent_avatar_id = state.get_player_avatar_id(&opponent_id)?;
        let controller_avatar = state.get_card(&controller_avatar_id);
        let opponent_avatar = state.get_card(&opponent_avatar_id);
        let controller_life = controller_avatar
            .get_unit_base()
            .map(|ub| ub.toughness)
            .unwrap_or_default()
            .saturating_sub(controller_avatar.get_damage_taken().unwrap_or_default());
        let opponent_life = opponent_avatar
            .get_unit_base()
            .map(|ub| ub.toughness)
            .unwrap_or_default()
            .saturating_sub(opponent_avatar.get_damage_taken().unwrap_or_default());

        let life_difference = controller_life.abs_diff(opponent_life);
        if paid_mana != life_difference as u8 {
            return Ok(vec![]);
        }

        Ok(vec![
            Effect::SetAvatarLife {
                player_id: controller_id,
                life: opponent_life,
            },
            Effect::SetAvatarLife {
                player_id: opponent_id,
                life: controller_life,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (TwistOfFate::NAME, |owner_id: PlayerId| {
    Box::new(TwistOfFate::new(owner_id))
});
