use crate::prelude::*;

#[derive(Debug, Clone)]
struct RollBoulder(uuid::Uuid);

#[async_trait::async_trait]
impl ActivatedAbility for RollBoulder {
    fn get_name(&self) -> String {
        "Tap to give Rolling Boulder a push".to_string()
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let boulder = state.get_card(&self.0);
        let picked_direction = pick_direction_source(
            player_id,
            &CARDINAL_DIRECTIONS,
            state,
            "Pick a direction to roll the Boulder",
            Some(self.0),
        )
        .await?;

        let mut last_zone = boulder.get_zone().clone();
        let mut effects = Vec::new();
        let units = CardQuery::new()
            .units()
            .id_not_in(vec![*card_id])
            .in_zone(&last_zone)
            .all(state);
        for unit in units {
            effects.push(Effect::TakeDamage {
                card_id: unit,
                from: *boulder.get_id(),
                damage: Damage::basic(4),
            });
        }

        while let Some(zone) = last_zone.zone_in_direction(&picked_direction, 1) {
            let units = CardQuery::new().units().in_zone(&zone).all(state);
            for unit in units {
                effects.push(Effect::MoveCard {
                    card_id: *boulder.get_id(),
                    from: (last_zone.clone())
                        .into_location()
                        .expect("MoveCard source must be a location"),
                    to: LocationQuery::from_zone(
                        (zone.clone()).with_region(boulder.get_region(state).clone()),
                    ),
                    player_id: boulder.get_controller_id(state),
                    tap: false,
                    through_path: None,
                });
                effects.push(Effect::TakeDamage {
                    card_id: unit,
                    from: *boulder.get_id(),
                    damage: Damage::basic(4),
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
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl RollingBoulder {
    pub const NAME: &'static str = "Rolling Boulder";
    pub const DESCRIPTION: &'static str = "Units here have “Tap → Give Rolling Boulder a push. It rolls as far as possible and deals 4 damage to each other unit along its path.”";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(4),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
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

    fn area_modifiers(&self, _state: &State) -> Vec<ContinuousEffect> {
        vec![ContinuousEffect::GrantActivatedAbility {
            ability: Box::new(RollBoulder(*self.get_id())),
            affected_cards: CardQuery::new().units().in_zone_of_card(self.get_id()),
        }]
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (RollingBoulder::NAME, |owner_id: PlayerId| {
        Box::new(RollingBoulder::new(owner_id))
    });
