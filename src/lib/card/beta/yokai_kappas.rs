use crate::prelude::*;

#[derive(Debug, Clone)]
struct DiscardWaterSiteToUntap;

#[async_trait::async_trait]
impl ActivatedAbility for DiscardWaterSiteToUntap {
    fn get_name(&self) -> String {
        "Discard a water site to untap".to_string()
    }

    fn can_activate(
        &self,
        _card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        Ok(CardQuery::new()
            .water_sites()
            .in_zone(Zone::Hand)
            .owned_by(player_id)
            .any(state))
    }

    fn get_cost(&self, card_id: &CardId, state: &State) -> anyhow::Result<Cost> {
        let controller_id = state.get_card(card_id).get_controller_id(state);
        Ok(Cost::additional_only(AdditionalCost::discard(
            CardQuery::new()
                .water_sites()
                .in_zone(Zone::Hand)
                .owned_by(&controller_id),
        )))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![
            Effect::SetTapped {
                card_id: *card_id,
                tapped: false,
            },
            Effect::SetCardData {
                card_id: *card_id,
                data: Arc::new(state.turns),
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct YokaiKappas {
    unit_base: UnitBase,
    card_base: CardBase,
    last_activation_on_turn: Option<usize>,
}

impl YokaiKappas {
    pub const NAME: &'static str = "Yokai Kappas";
    pub const DESCRIPTION: &'static str =
        "Discard a water site -> Untap Yokai Kappas. Use only once per turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            last_activation_on_turn: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for YokaiKappas {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(last_activation) = data.downcast_ref::<usize>() {
            self.last_activation_on_turn = Some(*last_activation);
        }

        Ok(())
    }

    fn get_additional_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if self.last_activation_on_turn == Some(state.turns) {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(DiscardWaterSiteToUntap)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (YokaiKappas::NAME, |owner_id: PlayerId| {
    Box::new(YokaiKappas::new(owner_id))
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yokai_kappas_ability_is_payable_while_tapped_and_summoning_sick() {
        let mut state = State::new_mock_state(vec![1]);
        let player_id = state.players[0].id;

        let mut kappas = YokaiKappas::new(player_id);
        let kappas_id = *kappas.get_id();
        kappas.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
        kappas.set_tapped(true);
        kappas.add_status(CardStatus::SummoningSickness);
        state.add_card(Box::new(kappas));

        let mut water_site = crate::card::SpringRiver::new(player_id);
        water_site.set_zone(Zone::Hand);
        state.add_card(Box::new(water_site));

        let ability = state
            .get_card(&kappas_id)
            .get_activated_abilities(&state)
            .expect("abilities to be available")
            .into_iter()
            .find(|ability| ability.get_name() == "Discard a water site to untap")
            .expect("Yokai Kappas ability to exist");

        assert!(
            ability
                .can_activate(&kappas_id, &player_id, &state)
                .expect("activation check should succeed")
        );
        assert!(
            ability
                .get_cost(&kappas_id, &state)
                .expect("cost should be available")
                .can_afford(&state, player_id)
                .expect("cost check should succeed")
        );
    }
}
