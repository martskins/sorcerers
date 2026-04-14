use crate::{
    card::{
        AdditionalCost, Artifact, ArtifactBase, ArtifactType, Card, CardBase, Cost, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_zone},
    state::{CardQuery, State},
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
        let picked_zone = pick_zone(
            player_id,
            &zones,
            state,
            false,
            "Pick a zone to shoot the payload at",
        )
        .await?;
        let units = picked_zone.get_units(state, None);
        let mana_cost = state
            .effect_log
            .iter()
            .find_map(|e| {
                if e.turn != state.turns {
                    return None;
                }

                match *e.effect {
                    Effect::DiscardCard {
                        player_id: pid,
                        card_id: cid,
                    } if &pid == player_id => {
                        let card = state.get_card(&cid);
                        Some(
                            card.get_costs(state)
                                .cloned()
                                .unwrap_or_default()
                                .mana_cost(),
                        )
                    }
                    _ => None,
                }
            })
            .unwrap_or_default();
        Ok(units
            .iter()
            .map(|unit| Effect::TakeDamage {
                card_id: unit.get_id().clone(),
                damage: mana_cost.into(),
                from: card_id.clone(),
                is_strike: false,
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
                        CardQuery::from_id(bearer_id.clone()).untapped(),
                    ))
                    .with_additional(AdditionalCost::tap(
                        CardQuery::new()
                            .untapped()
                            .in_zone(bearer.get_zone())
                            .units()
                            .controlled_by(&bearer.get_controller_id(state)),
                    ))
                    .with_additional(AdditionalCost::discard(
                        CardQuery::new()
                            .in_zone(&Zone::Hand)
                            .controlled_by(&bearer.get_controller_id(state)),
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
    pub const DESCRIPTION: &'static str = "Tap bearer and another ally here, Discard a card → Deal damage equal to the discarded card's mana cost to each unit at target location up to three steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Weapon],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(5),
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

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ShootPayload)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (PayloadTrebuchet::NAME, |owner_id: PlayerId| {
        Box::new(PayloadTrebuchet::new(owner_id))
    });
