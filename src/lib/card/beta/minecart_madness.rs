use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MinecartMadness {
    card_base: CardBase,
}

impl MinecartMadness {
    pub const NAME: &'static str = "Minecart Madness";
    pub const DESCRIPTION: &'static str = "This turn, your units can move between any sites in a chosen span of land as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for MinecartMadness {
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
impl Magic for MinecartMadness {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let spans_of_land = state
            .get_spans_of_land()
            .into_iter()
            .map(|span| span.iter().map(Zone::from).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        if spans_of_land.is_empty() {
            return Ok(vec![]);
        }

        let span = pick_zone_group(
            controller_id,
            &spans_of_land,
            state,
            false,
            "Minecart Madness: Pick a span of land",
        )
        .await?;

        Ok(vec![Effect::AddTemporaryEffect {
            effect: TemporaryEffect::ConnectSites {
                sites: span,
                affected_cards: CardQuery::new().units().controlled_by(&controller_id),
                expires_on_effect: Box::new(EffectQuery::TurnEnd {
                    player_id: Some(controller_id),
                }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MinecartMadness::NAME, |owner_id: PlayerId| {
        Box::new(MinecartMadness::new(owner_id))
    });
