use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Blink {
    card_base: CardBase,
}

impl Blink {
    pub const NAME: &'static str = "Blink";
    pub const DESCRIPTION: &'static str =
        "An ally teleports to a location it’s nearby. Draw a card.";

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
impl Card for Blink {
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
impl Magic for Blink {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let controller_id = caster.get_controller_id(state);
        let card_id = CardQuery::new()
            .units()
            .controlled_by(&controller_id)
            .with_prompt("Pick an ally to teleport")
            .with_source_card(*self.get_id())
            .id_not_in(vec![*caster_id])
            .pick(&controller_id, state)
            .await?;
        let card_id = card_id.expect("value not to be None");
        let card = state.get_card(&card_id);
        let location = LocationQuery::new()
            .near(&Zone::Location(card.get_location().clone()))
            .with_prompt("Pick a zone to teleport to")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state)
            .await?;

        Ok(vec![
            Effect::TeleportCard {
                player_id: controller_id,
                card_id,
                to_location: location,
            },
            Effect::DrawCard {
                player_id: self.get_controller_id(state),
                count: 1,
                kind: DrawKind::Choice,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Blink::NAME, |owner_id: PlayerId| {
    Box::new(Blink::new(owner_id))
});
