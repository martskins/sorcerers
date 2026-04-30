use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::{AbilityCounter, Effect},
    game::{PlayerId, pick_option},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MonasteryGargoyle {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MonasteryGargoyle {
    pub const NAME: &'static str = "Monastery Gargoyle";
    pub const DESCRIPTION: &'static str = "At the start and end of your turn, choose whether Monastery Gargoyle has Airborne or is a Monument (Immobile).";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "E"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    async fn toggle_form(
        card_id: uuid::Uuid,
        controller_id: uuid::Uuid,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let options = vec!["Airborne".to_string(), "Monument (Immobile)".to_string()];
        let picked = pick_option(
            &controller_id,
            &options,
            state,
            "Monastery Gargoyle: Choose form",
            false,
        )
        .await?;

        let chosen_ability = if picked == 0 {
            Ability::Airborne
        } else {
            Ability::Immobile
        };
        let removed_ability = if picked == 0 {
            Ability::Immobile
        } else {
            Ability::Airborne
        };

        Ok(vec![
            Effect::RemoveAbility {
                card_id,
                modifier: removed_ability,
            },
            Effect::RemoveAbility {
                card_id,
                modifier: chosen_ability.clone(),
            },
            Effect::AddAbilityCounter {
                card_id,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: chosen_ability,
                    expires_on_effect: None,
                },
            },
        ])
    }
}

#[async_trait::async_trait]
impl Card for MonasteryGargoyle {
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

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        Self::toggle_form(*self.get_id(), controller_id, state).await
    }

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        Self::toggle_form(*self.get_id(), controller_id, state).await
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MonasteryGargoyle::NAME, |owner_id: PlayerId| {
        Box::new(MonasteryGargoyle::new(owner_id))
    });
