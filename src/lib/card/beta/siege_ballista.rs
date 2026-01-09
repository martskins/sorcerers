use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{CardAction, PlayerId, Thresholds, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
enum SiegeBallistaAbility {
    TapToDealDamage,
}

#[async_trait::async_trait]
impl CardAction for SiegeBallistaAbility {
    fn get_name(&self) -> &str {
        match self {
            SiegeBallistaAbility::TapToDealDamage => "Tap to deal 3 damage to target unit",
        }
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<crate::effect::Effect>> {
        match self {
            SiegeBallistaAbility::TapToDealDamage => {
                let card = state
                    .get_card(card_id)
                    .get_artifact()
                    .ok_or(anyhow::anyhow!("Card is not an artifact"))?;
                if let Some(bearer_id) = card.get_bearer()? {
                    let bearer = state.get_card(&bearer_id);
                    if bearer.is_tapped() {
                        return Ok(vec![]);
                    }

                    let zone = bearer.get_zone();
                    let untapped_allies_here: Vec<uuid::Uuid> = zone
                        .get_units(state, Some(card.get_controller_id()))
                        .iter()
                        .filter(|c| c.get_id() != &bearer_id)
                        .filter(|c| !c.is_tapped())
                        .map(|c| c.get_id())
                        .cloned()
                        .collect();

                    let picked_ally_id = pick_card(
                        card.get_controller_id(),
                        &untapped_allies_here,
                        state,
                        "Siege Ballista: Pick an ally to tap",
                    )
                    .await?;

                    let valid_targets: Vec<uuid::Uuid> = bearer
                        .get_zones_within_steps(state, 2)
                        .iter()
                        .flat_map(|z| z.get_units(state, None))
                        .filter(|c| c.is_unit())
                        .map(|c| c.get_id())
                        .cloned()
                        .collect();

                    let picked_unit_id = pick_card(
                        card.get_controller_id(),
                        &valid_targets,
                        state,
                        "Siege Ballista: Pick a unit to deal 3 damage to",
                    )
                    .await?;

                    return Ok(vec![
                        Effect::TapCard { card_id: bearer_id },
                        Effect::TapCard {
                            card_id: picked_ally_id,
                        },
                        Effect::TakeDamage {
                            card_id: picked_unit_id,
                            from: bearer_id,
                            damage: 3,
                        },
                    ]);
                }

                Ok(vec![])
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiegeBallista {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl SiegeBallista {
    pub const NAME: &'static str = "Siege Ballista";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase { attached_to: None },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse(""),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for SiegeBallista {}

#[async_trait::async_trait]
impl Card for SiegeBallista {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }

    fn get_actions(&self, state: &State) -> anyhow::Result<Vec<Box<dyn CardAction>>> {
        if let Some(bearer_id) = self.get_bearer()? {
            let bearer = state.get_card(&bearer_id);
            if bearer.is_tapped() {
                return Ok(vec![]);
            }

            let zone = bearer.get_zone();
            let untapped_allies_here = zone
                .get_units(state, Some(self.get_controller_id()))
                .iter()
                .filter(|c| c.get_id() != &bearer_id)
                .filter(|c| !c.is_tapped())
                .map(|c| c.get_id())
                .count();

            if untapped_allies_here > 0 {
                return Ok(vec![Box::new(SiegeBallistaAbility::TapToDealDamage)]);
            }

            return Ok(vec![]);
        }

        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SiegeBallista::NAME, |owner_id: PlayerId| {
    Box::new(SiegeBallista::new(owner_id))
});