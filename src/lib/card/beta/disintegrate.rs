use crate::{
    card::{Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    state::{CardMatcher, State},
};

/// **Disintegrate** — Elite Fire Magic (3 cost, FF threshold)
///
/// Banish target minion nearby, and everything it carries.
#[derive(Debug, Clone)]
pub struct Disintegrate {
    pub card_base: CardBase,
}

impl Disintegrate {
    pub const NAME: &'static str = "Disintegrate";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_mana_and_threshold(3, "FF"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Disintegrate {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let caster_zone = state.get_card(caster_id).get_zone().clone();
        let controller_id = self.get_controller_id(state);

        // Collect all minions in or near the caster's zone.
        let valid_targets: Vec<uuid::Uuid> = CardMatcher::minions_near(&caster_zone).resolve_ids(state);

        if valid_targets.is_empty() {
            return Ok(vec![]);
        }

        let target_id = pick_card(
            &controller_id,
            &valid_targets,
            state,
            "Disintegrate: Choose a minion to banish",
        )
        .await?;

        let target = state.get_card(&target_id);
        let target_zone = target.get_zone().clone();

        // Banish the target and every artifact it carries.
        let mut effects: Vec<Effect> = vec![Effect::BanishCard {
            card_id: target_id.clone(),
            from: target_zone,
        }];

        let carried: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_artifact())
            .filter(|c| {
                c.get_artifact_base()
                    .and_then(|ab| ab.bearer.as_ref())
                    .map(|bearer| bearer == &target_id)
                    .unwrap_or(false)
            })
            .map(|c| c.get_id().clone())
            .collect();

        for artifact_id in carried {
            let artifact = state.get_card(&artifact_id);
            effects.push(Effect::BanishCard {
                card_id: artifact_id.clone(),
                from: artifact.get_zone().clone(),
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Disintegrate::NAME, |owner_id: PlayerId| {
    Box::new(Disintegrate::new(owner_id))
});
