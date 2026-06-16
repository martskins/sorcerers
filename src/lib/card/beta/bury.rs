use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Bury {
    card_base: CardBase,
}

impl Bury {
    pub const NAME: &'static str = "Bury";
    pub const DESCRIPTION: &'static str = "Burrow target minion or artifact, if able.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
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
impl Card for Bury {
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
impl Magic for Bury {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let player_id = self.get_controller_id(state);
        let Some(picked_card_id) = CardQuery::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .with_source_card(*self.get_id())
            .with_prompt("Pick a minion or artifact to bury")
            .pick(&player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        let picked_card = state.get_card(&picked_card_id);
        Ok(vec![Effect::MoveCard {
            player_id: self.get_controller_id(state),
            card_id: picked_card_id,
            from: picked_card.get_location().clone(),
            to: picked_card
                .get_location()
                .with_region(Region::Underground)
                .into(),
            tap: false,
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Bury::NAME, |owner_id: PlayerId| {
    Box::new(Bury::new(owner_id))
});
