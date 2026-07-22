use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MonasteryGargoyle {
    unit_base: UnitBase,
    artifact_base: Option<ArtifactBase>,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;
const TURN_START_HOOK: HookId = 2;

impl MonasteryGargoyle {
    pub const NAME: &'static str = "Monastery Gargoyle";
    pub const DESCRIPTION: &'static str = "At the start and end of your turn, choose whether Monastery Gargoyle has Airborne or is a Monument.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            artifact_base: None,
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "E"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    async fn toggle_form(
        card_id: CardId,
        controller_id: uuid::Uuid,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let options = vec!["Airborne".to_string(), "Monument".to_string()];
        let picked = pick_option(
            &controller_id,
            &options,
            state,
            "Choose form",
            false,
            Some(card_id),
        )
        .await?;

        if picked == 0 {
            Ok(vec![
                Effect::SetArtifactBase {
                    card_id,
                    artifact_base: None,
                },
                Effect::RemoveAbility {
                    card_id,
                    modifier: Ability::Airborne,
                },
                Effect::AddAbilityCounter {
                    card_id,
                    counter: AbilityCounter {
                        id: uuid::Uuid::new_v4(),
                        ability: Ability::Airborne,
                        expires_on_effect: None,
                    },
                },
            ])
        } else {
            Ok(vec![
                Effect::RemoveAbility {
                    card_id,
                    modifier: Ability::Airborne,
                },
                Effect::SetArtifactBase {
                    card_id,
                    artifact_base: Some(ArtifactBase {
                        types: vec![ArtifactType::Monument],
                        ..Default::default()
                    }),
                },
            ])
        }
    }
}

impl Artifact for MonasteryGargoyle {}

#[async_trait::async_trait]
impl Card for MonasteryGargoyle {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        self.artifact_base.is_none().then_some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        self.artifact_base.is_none().then_some(&mut self.unit_base)
    }

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        self.artifact_base.as_ref()
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        self.artifact_base.as_mut()
    }

    fn set_artifact_base(&mut self, artifact_base: Option<ArtifactBase>) {
        let tapped = self
            .artifact_base
            .as_ref()
            .map(|base| base.tapped)
            .unwrap_or(self.unit_base.tapped);

        match artifact_base {
            Some(mut artifact_base) => {
                artifact_base.tapped = tapped;
                self.artifact_base = Some(artifact_base);
            }
            None => {
                self.unit_base.tapped = tapped;
                self.artifact_base = None;
            }
        }
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        self.artifact_base.is_some().then_some(self)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![
            Hook {
                id: TURN_END_HOOK,
                trigger: EffectQuery::TurnEnd { player_id: None },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
            Hook {
                id: TURN_START_HOOK,
                trigger: EffectQuery::TurnStart { player_id: None },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
        ])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TURN_END_HOOK | TURN_START_HOOK => {
                let controller_id = self.get_controller_id(state);
                if state.current_player() != controller_id {
                    return Ok(vec![]);
                }
                if !self.get_zone().is_in_play() {
                    return Ok(vec![]);
                }
                Self::toggle_form(*self.get_id(), controller_id, state).await
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MonasteryGargoyle::NAME, |owner_id: PlayerId| {
        Box::new(MonasteryGargoyle::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        card::{CardType, Sorcerer},
        deck::Deck,
        networking::message::{ClientMessage, ServerMessage},
        query::{CardQuery, QueryCache},
        state::{Player, PlayerWithDeck},
    };

    fn test_state() -> (
        State,
        PlayerId,
        CardId,
        async_channel::Sender<ClientMessage>,
        async_channel::Receiver<ServerMessage>,
    ) {
        let player_id = uuid::Uuid::new_v4();
        let opponent_id = uuid::Uuid::new_v4();
        let avatar = Sorcerer::new(player_id);
        let avatar_id = *avatar.get_id();
        let opponent_avatar = Sorcerer::new(opponent_id);
        let opponent_avatar_id = *opponent_avatar.get_id();

        let mut gargoyle = MonasteryGargoyle::new(player_id);
        let gargoyle_id = *gargoyle.get_id();
        gargoyle.set_zone(Zone::Location(Location::Square(1, Region::Surface)));

        let player = PlayerWithDeck {
            player: Player {
                id: player_id,
                name: "Player 1".to_string(),
            },
            deck: Deck::new(
                &player_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_id,
            ),
            cards: vec![Box::new(avatar), Box::new(gargoyle)],
        };
        let opponent = PlayerWithDeck {
            player: Player {
                id: opponent_id,
                name: "Player 2".to_string(),
            },
            deck: Deck::new(
                &opponent_id,
                "Opponent Deck".to_string(),
                vec![],
                vec![],
                opponent_avatar_id,
            ),
            cards: vec![Box::new(opponent_avatar)],
        };

        let (server_tx, server_rx) = async_channel::unbounded();
        let (client_tx, client_rx) = async_channel::unbounded();
        let state = State::new(
            uuid::Uuid::new_v4(),
            vec![player, opponent],
            server_tx,
            client_rx,
        );

        (state, player_id, gargoyle_id, client_tx, server_rx)
    }

    async fn choose_form(
        state: &mut State,
        player_id: PlayerId,
        gargoyle_id: CardId,
        client_tx: async_channel::Sender<ClientMessage>,
        server_rx: async_channel::Receiver<ServerMessage>,
        action_idx: usize,
    ) {
        let game_id = state.game_id;
        let _server_rx_keepalive = server_rx.clone();
        let responder = tokio::spawn(async move {
            let message = server_rx.recv().await.unwrap();
            match message {
                ServerMessage::PickAction {
                    player_id, actions, ..
                } => {
                    assert_eq!(actions, vec!["Airborne", "Monument"]);
                    client_tx
                        .send(ClientMessage::PickAction {
                            game_id,
                            player_id,
                            action_idx,
                        })
                        .await
                        .unwrap();
                }
                other => panic!("expected form choice, got {:?}", other),
            }
        });

        let effects = MonasteryGargoyle::toggle_form(gargoyle_id, player_id, state)
            .await
            .unwrap();
        responder.await.unwrap();
        for effect in effects {
            effect.apply(state).await.unwrap();
        }
    }

    #[tokio::test]
    async fn monument_form_is_a_real_monument_artifact() {
        QueryCache::init();

        let (mut state, player_id, gargoyle_id, client_tx, server_rx) = test_state();
        choose_form(&mut state, player_id, gargoyle_id, client_tx, server_rx, 1).await;

        let gargoyle = state.get_card(&gargoyle_id);
        assert_eq!(gargoyle.get_card_type(), CardType::Artifact);
        assert!(!gargoyle.is_unit());
        assert!(gargoyle.is_artifact());
        assert!(!gargoyle.has_ability(&state, &Ability::Airborne));
        assert!(!gargoyle.get_artifact().unwrap().can_be_carried());

        assert!(
            CardQuery::new()
                .artifacts()
                .artifact_type(ArtifactType::Monument)
                .all(&state)
                .contains(&gargoyle_id)
        );
        assert!(
            !CardQuery::new()
                .minions()
                .all(&state)
                .contains(&gargoyle_id)
        );
    }

    #[tokio::test]
    async fn airborne_form_removes_monument_artifact_form() {
        QueryCache::init();

        let (mut state, player_id, gargoyle_id, client_tx, server_rx) = test_state();
        choose_form(&mut state, player_id, gargoyle_id, client_tx, server_rx, 1).await;

        let (client_tx, server_rx) = {
            let (server_tx, server_rx) = async_channel::unbounded();
            let (client_tx, client_rx) = async_channel::unbounded();
            state.server_tx = server_tx;
            state.client_rx = client_rx;
            (client_tx, server_rx)
        };
        choose_form(&mut state, player_id, gargoyle_id, client_tx, server_rx, 0).await;

        let gargoyle = state.get_card(&gargoyle_id);
        assert_eq!(gargoyle.get_card_type(), CardType::Minion);
        assert!(gargoyle.is_unit());
        assert!(!gargoyle.is_artifact());
        assert!(gargoyle.has_ability(&state, &Ability::Airborne));
        assert!(
            !CardQuery::new()
                .artifacts()
                .artifact_type(ArtifactType::Monument)
                .all(&state)
                .contains(&gargoyle_id)
        );
    }
}
