use crate::{
    card::{
        AdditionalCost, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardType, Cost, Costs, Edition, Rarity,
        Region, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_zone},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
struct ShootPayload;

#[async_trait::async_trait]
impl ActivatedAbility for ShootPayload {
    fn get_name(&self) -> String {
        "Shoot Payload".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let zones = state.get_card(card_id).get_zones_within_steps(state, 3);
        let picked_zone = pick_zone(player_id, &zones, state, false, "Pick a zone to shoot the payload at").await?;
        let units = picked_zone.get_units(state, None);

        Ok(units
            .iter()
            .map(|unit| Effect::TakeDamage {
                card_id: unit.get_id().clone(),
                damage: 3,
                from: card_id.clone(),
            })
            .collect())
    }

    fn get_cost(&self, card_id: &uuid::Uuid, state: &State) -> anyhow::Result<Cost> {
        let bearer_id = state.get_card(card_id).get_base().bearer;
        match bearer_id {
            Some(bearer_id) => {
                let bearer = state.get_card(&bearer_id);
                Ok(Cost::ZERO
                    .with_additional(AdditionalCost::tap(
                        CardMatcher::from_id(bearer_id.clone()).with_tapped(false),
                    ))
                    .with_additional(AdditionalCost::tap(
                        CardMatcher::new()
                            .with_tapped(false)
                            .with_zone(bearer.get_zone())
                            .with_card_types(vec![CardType::Minion, CardType::Avatar])
                            .with_controller_id(&bearer.get_controller_id(state)),
                    ))
                    .with_additional(AdditionalCost::discard(
                        CardMatcher::new()
                            .with_zone(&Zone::Hand)
                            .with_controller_id(&bearer.get_controller_id(state)),
                    )))
            }
            None => Ok(Cost::ZERO),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PayloadTrebuchet {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl PayloadTrebuchet {
    pub const NAME: &'static str = "Payload Trebuchet";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Weapon],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(5),
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

impl Artifact for PayloadTrebuchet {}

#[async_trait::async_trait]
impl Card for PayloadTrebuchet {
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

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ShootPayload)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (PayloadTrebuchet::NAME, |owner_id: PlayerId| {
    Box::new(PayloadTrebuchet::new(owner_id))
});
