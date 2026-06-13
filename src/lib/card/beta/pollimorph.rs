use crate::prelude::*;

/// **Pollimorph** — Elite Magic (2 cost, WW threshold)
///
/// Transform target nearby minion into a Frog token.
#[derive(Debug, Clone)]
pub struct Pollimorph {
    card_base: CardBase,
}

impl Pollimorph {
    pub const NAME: &'static str = "Pollimorph";
    pub const DESCRIPTION: &'static str = "Transform target nearby minion into a Frog token.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "WW"),
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
impl Card for Pollimorph {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for Pollimorph {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster_location = state.get_card(caster_id).get_location().clone();

        let prompt = "Pick a minion to transform into a Frog";
        let Some(target_id) = CardQuery::new()
            .minions()
            .near_to(&caster_location)
            .with_prompt(prompt)
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let target_loc = state.get_card(&target_id).get_location().clone();
        let target_controller = state.get_card(&target_id).get_controller_id(state);

        Ok(vec![
            Effect::BuryCard { card_id: target_id },
            Effect::SummonToken {
                player_id: target_controller,
                token_type: TokenType::Frog,
                location: target_loc,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Pollimorph::NAME, |owner_id: PlayerId| {
    Box::new(Pollimorph::new(owner_id))
});
