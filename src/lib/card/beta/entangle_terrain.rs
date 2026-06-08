use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct EntangleTerrain {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl EntangleTerrain {
    pub const NAME: &'static str = "Entangle Terrain";
    pub const DESCRIPTION: &'static str =
        "Minions occupying affected sites lose Airborne and are Immobile. Lasts 3 of your turns.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "EE"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
        }
    }
}

impl Aura for EntangleTerrain {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        let controller_id = self.get_controller_id(state);
        let turns_in_play = state
            .effect_log()
            .iter()
            .skip_while(|e| !matches!(e.effect, Effect::PlayCard { ref card_id, .. } if card_id == self.get_id()))
            .filter(|e| matches!(e.effect, Effect::EndTurn { ref player_id, .. } if player_id == &controller_id))
            .count();

        Ok(turns_in_play >= 3)
    }
}

#[async_trait::async_trait]
impl Card for EntangleTerrain {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let affected_cards = CardQuery::new()
            .minions()
            .in_affected_zones_of_card(self.get_id());
        Ok(vec![
            OngoingEffect::GrantAbility {
                ability: Ability::Immobile,
                affected_cards: affected_cards.clone(),
            },
            OngoingEffect::RemoveAbilities {
                removal: AbilityRemoval::exact(Ability::Airborne),
                affected_cards,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (EntangleTerrain::NAME, |owner_id: PlayerId| {
        Box::new(EntangleTerrain::new(owner_id))
    });
