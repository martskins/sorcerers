use crate::prelude::*;

#[derive(Debug, Clone)]
struct TapToStrikeNearbyMinions;

#[async_trait::async_trait]
impl ActivatedAbility for TapToStrikeNearbyMinions {
    fn get_name(&self) -> String {
        "Tap -> Strike nearby units".to_string()
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let kraken = state.get_card(card_id);
        let mut effects = vec![];
        for unit_id in CardQuery::new()
            .units()
            .near_to(kraken.get_location())
            .id_not(*kraken.get_id())
            .all(state)
        {
            effects.push(Effect::strike(state, *kraken.get_id(), unit_id)?);
        }

        effects.push(Effect::MoveCard {
            player_id: *player_id,
            card_id: *card_id,
            from: kraken.get_location().clone(),
            to: kraken
                .get_location()
                .with_region(kraken.get_region(state).clone())
                .into(),
            tap: false,
            through_path: None,
        });

        Ok(effects)
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::surface(card_id)))
    }
}

#[derive(Debug, Clone)]
pub struct DiluvianKraken {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl DiluvianKraken {
    pub const NAME: &'static str = "Diluvian Kraken";
    pub const DESCRIPTION: &'static str =
        "Submerge\r \r Tap → Surface to strike each other unit nearby.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 8,
                toughness: 8,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Monster],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(8, "WWW"),
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
impl Card for DiluvianKraken {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(TapToStrikeNearbyMinions)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (DiluvianKraken::NAME, |owner_id: PlayerId| {
        Box::new(DiluvianKraken::new(owner_id))
    });
