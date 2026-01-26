use crate::{
    card::{
        AdditionalCost, AreaModifiers, Artifact, ArtifactBase, Card, CardBase, Cost, Edition, Rarity, Region, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, CARDINAL_DIRECTIONS, PlayerId, pick_direction},
    query::{CardQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
struct RollBoulder(uuid::Uuid);

#[async_trait::async_trait]
impl ActivatedAbility for RollBoulder {
    fn get_name(&self) -> String {
        "Tap to give Rolling Boulder a push".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost {
            additional: vec![AdditionalCost::Tap {
                card: CardQuery::Specific {
                    id: uuid::Uuid::new_v4(),
                    card_id: card_id.clone(),
                },
            }],
            ..Default::default()
        })
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let boulder = state.get_card(&self.0);
        let picked_direction = pick_direction(
            player_id,
            &CARDINAL_DIRECTIONS,
            state,
            "Pick a direction to roll the Boulder",
        )
        .await?;

        let mut last_zone = boulder.get_zone().clone();
        let mut effects = Vec::new();
        for unit in last_zone.get_units(state, None) {
            if unit.get_id() == card_id {
                continue;
            }

            effects.push(Effect::TakeDamage {
                card_id: unit.get_id().clone(),
                from: boulder.get_id().clone(),
                damage: 4,
            });
        }

        while let Some(zone) = last_zone.zone_in_direction(&picked_direction, 1) {
            let units = zone.get_units(state, None);
            for unit in units {
                effects.push(Effect::MoveCard {
                    card_id: boulder.get_id().clone(),
                    from: last_zone.clone(),
                    to: ZoneQuery::Specific {
                        id: uuid::Uuid::new_v4(),
                        zone: zone.clone(),
                    },
                    player_id: boulder.get_controller_id(state).clone(),
                    tap: false,
                    region: Region::Surface,
                    through_path: None,
                });
                effects.push(Effect::TakeDamage {
                    card_id: unit.get_id().clone(),
                    from: boulder.get_id().clone(),
                    damage: 4,
                });
            }

            last_zone = zone.clone();
        }

        // reverse the effects vec so that they are applied in FIFO order
        effects.reverse();
        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct RollingBoulder {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl RollingBoulder {
    pub const NAME: &'static str = "Rolling Boulder";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                bearer: None,
                needs_bearer: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, ""),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for RollingBoulder {}

#[async_trait::async_trait]
impl Card for RollingBoulder {
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let granted_activated_abilities = self
            .get_zone()
            .get_units(state, None)
            .iter()
            .map(|u| {
                (
                    u.get_id().clone(),
                    vec![Box::new(RollBoulder(self.get_id().clone())) as Box<dyn ActivatedAbility>],
                )
            })
            .collect();
        AreaModifiers {
            grants_activated_abilities: granted_activated_abilities,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (RollingBoulder::NAME, |owner_id: PlayerId| {
    Box::new(RollingBoulder::new(owner_id))
});