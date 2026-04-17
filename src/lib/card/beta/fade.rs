use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Fade {
    card_base: CardBase,
}

impl Fade {
    pub const NAME: &'static str = "Fade";
    pub const DESCRIPTION: &'static str =
        "Give an allied minion Stealth. If it occupies an enemy site, draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Fade {
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
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(target_id) = CardQuery::new()
            .minions()
            .controlled_by(&controller_id)
            .in_play()
            .with_prompt("Fade: Pick an allied minion to give Stealth")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let mut effects = vec![Effect::AddAbilityCounter {
            card_id: target_id,
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Stealth,
                expires_on_effect: None,
            },
        }];

        let target = state.get_card(&target_id);
        let target_zone = target.get_zone();
        let on_enemy_site = target_zone
            .get_site(state)
            .is_some_and(|site| site.get_controller_id(state) != controller_id);

        if on_enemy_site {
            effects.push(Effect::DrawCard {
                player_id: controller_id,
                count: 1,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Fade::NAME, |owner_id: PlayerId| {
    Box::new(Fade::new(owner_id))
});
