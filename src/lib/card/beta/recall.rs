use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

/// **Recall** — Exceptional Magic (2 cost, AA threshold)
///
/// Teleport any number of allied minions to the caster's location.
#[derive(Debug, Clone)]
pub struct Recall {
    card_base: CardBase,
}

impl Recall {
    pub const NAME: &'static str = "Recall";
    pub const DESCRIPTION: &'static str =
        "Teleport any number of allied minions to the caster's location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "AA"),
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
impl Card for Recall {
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
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster_zone = state.get_card(caster_id).get_zone().clone();

        let allied_minions: Vec<uuid::Uuid> = CardQuery::new()
            .minions()
            .in_play()
            .controlled_by(&controller_id)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_zone() != &caster_zone)
            .collect();

        if allied_minions.is_empty() {
            return Ok(vec![]);
        }

        let mut effects = vec![];
        let mut remaining = allied_minions;
        loop {
            if remaining.is_empty() {
                break;
            }
            let prompt = "Recall: Pick an allied minion to teleport (or cancel)";
            let picked = CardQuery::from_ids(remaining.clone())
                .with_prompt(prompt)
                .pick(&controller_id, state, false)
                .await?;

            match picked {
                Some(card_id) => {
                    effects.push(Effect::TeleportCard {
                        player_id: controller_id,
                        card_id,
                        to_zone: caster_zone.clone(),
                    });
                    remaining.retain(|id| *id != card_id);
                }
                None => break,
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Recall::NAME, |owner_id: PlayerId| {
    Box::new(Recall::new(owner_id))
});
