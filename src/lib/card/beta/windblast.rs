use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Windblast {
    card_base: CardBase,
}

impl Windblast {
    pub const NAME: &'static str = "Windblast";
    pub const DESCRIPTION: &'static str =
        "Push everything atop sites one step in a cardinal direction.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Windblast {
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
impl Magic for Windblast {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let direction = pick_direction(
            &controller_id,
            &CARDINAL_DIRECTIONS,
            state,
            "Windblast: Pick a direction",
            *self.get_id(),
        )
        .await?;

        let mut effects = vec![];
        let cards = CardQuery::new().in_play().all(state);
        for card_id in cards {
            let card = state.get_card(&card_id);
            if card.get_region(state) != &Region::Surface
                || card.get_location().get_site(state).is_none()
            {
                continue;
            }

            let from_location = card.get_location();
            let Some(to_location) =
                from_location.step_in_direction(&direction, state, Some(&card_id))
            else {
                continue;
            };
            if to_location.get_site(state).is_none() {
                continue;
            }

            effects.push(Effect::MoveCard {
                player_id: card.get_controller_id(state),
                card_id,
                from: from_location.clone(),
                to: LocationQuery::from_location(to_location.with_region(Region::Surface)),
                tap: false,
                through_path: None,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Windblast::NAME, |owner_id: PlayerId| {
    Box::new(Windblast::new(owner_id))
});
