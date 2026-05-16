use super::*;

impl Game {
    pub fn process_message(&mut self, message: &ServerMessage) -> Option<Scene> {
        match message {
            ServerMessage::MulligansEnded => {
                self.data.status = Status::Idle;
                None
            }
            ServerMessage::PlaySoundEffect { .. } => {
                if let Ok(sound_data) = StaticSoundData::from_file("assets/sounds/play_card.mp3") {
                    self.audio_manager.play(sound_data).ok();
                }
                None
            }
            ServerMessage::PlayerDisconnected { player_id, .. } => {
                self.data.status = Status::GameAborted {
                    reason: format!("Player {} disconnected.", player_id),
                };
                None
            }
            ServerMessage::Resume { .. } => {
                self.data.status = Status::Idle;
                None
            }
            ServerMessage::Wait { prompt, .. } => {
                self.data.status = Status::Waiting {
                    prompt: prompt.clone(),
                };
                None
            }
            ServerMessage::LogEvent {
                id,
                description,
                datetime,
            } => {
                self.data.events.push(Event {
                    id: *id,
                    description: description.clone(),
                    datetime: *datetime,
                });
                self.push_toast(CardToast::new_event(description.clone()));
                None
            }
            ServerMessage::PickZoneGroup {
                groups: zones,
                prompt,
                ..
            } => {
                self.data.status = Status::SelectingZoneGroup {
                    groups: zones.clone(),
                    prompt: prompt.clone(),
                };
                None
            }
            ServerMessage::PickZone { zones, prompt, .. } => {
                self.data.status = Status::SelectingZone {
                    zones: zones.clone(),
                    prompt: prompt.clone(),
                };
                None
            }
            ServerMessage::PlayableZones { card_id, zones, .. } => {
                self.data.status = Status::PreviewingPlayableZones {
                    card_id: *card_id,
                    zones: zones.clone(),
                };
                None
            }
            ServerMessage::PickAmount {
                prompt,
                min_amount,
                max_amount,
                ..
            } => {
                self.data.status = Status::SelectingAmount {
                    min_amount: *min_amount,
                    max_amount: *max_amount,
                    prompt: prompt.clone(),
                };
                None
            }
            ServerMessage::PickPath { paths, prompt, .. } => {
                self.data.status = Status::SelectingPath {
                    paths: paths.clone(),
                    prompt: prompt.clone(),
                };
                None
            }
            ServerMessage::RevealCards {
                cards,
                action,
                prompt,
                ..
            } => {
                let renderables = self
                    .data
                    .cards
                    .iter()
                    .filter(|c| cards.contains(&c.id))
                    .collect();
                self.overlay = Some(Box::new(ActionOverlay::new(
                    self.client.clone(),
                    &self.game_id,
                    renderables,
                    &self.data.player_id,
                    prompt.to_string(),
                    action.clone(),
                )));
                None
            }
            ServerMessage::CardPlayed {
                card_id,
                description,
            } => {
                if let Some(card) = self.data.cards.iter().find(|c| c.id == *card_id).cloned() {
                    self.push_toast(CardToast::new_card(card, description.clone()));
                }
                None
            }
            ServerMessage::PickCards {
                cards,
                prompt,
                preview,
                ..
            } => {
                self.data.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: *preview,
                    prompt: prompt.clone(),
                    multiple: true,
                };
                if *preview {
                    let renderables = self
                        .data
                        .cards
                        .iter()
                        .filter(|c| cards.contains(&c.id))
                        .collect();
                    self.overlay = Some(Box::new(SelectionOverlay::new(
                        self.client.clone(),
                        &self.game_id,
                        &self.data.player_id,
                        renderables,
                        cards.clone(),
                        prompt,
                        SelectionOverlayBehaviour::Pick,
                    )));
                }
                None
            }
            ServerMessage::DistributeDamage {
                player_id,
                attacker,
                defenders,
                damage,
            } => {
                self.data.status = Status::DistributingDamage {
                    player_id: *player_id,
                    attacker: *attacker,
                    defenders: defenders.clone(),
                    damage: *damage,
                };
                let defenders_data: Vec<CardData> = self
                    .data
                    .cards
                    .iter()
                    .filter(|c| defenders.contains(&c.id))
                    .cloned()
                    .collect();
                if let Some(attacker_data) =
                    self.data.cards.iter().find(|c| c.id == *attacker).cloned()
                {
                    self.overlay = Some(Box::new(CombatResolutionOverlay::new(
                        self.client.clone(),
                        &self.game_id,
                        player_id,
                        attacker_data,
                        defenders_data,
                        *damage,
                    )));
                }
                None
            }
            ServerMessage::PickCard {
                cards,
                pickable_cards,
                prompt,
                preview,
                ..
            } => {
                self.data.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: *preview,
                    prompt: prompt.clone(),
                    multiple: false,
                };

                if let Err(e) = self.open_viewers(cards) {
                    eprintln!("Failed to compute viewers for card selection: {}", e);
                }

                if *preview {
                    let renderables = self
                        .data
                        .cards
                        .iter()
                        .filter(|c| cards.contains(&c.id))
                        .collect();
                    self.overlay = Some(Box::new(SelectionOverlay::new(
                        self.client.clone(),
                        &self.game_id,
                        &self.data.player_id,
                        renderables,
                        pickable_cards.clone(),
                        prompt,
                        SelectionOverlayBehaviour::Pick,
                    )));
                }
                None
            }
            ServerMessage::PickAction {
                prompt,
                actions,
                anchor_on_cursor,
                ..
            } => {
                self.data.status = Status::SelectingAction {
                    prompt: prompt.to_string(),
                    actions: actions.clone(),
                    anchor_on_cursor: *anchor_on_cursor,
                };
                None
            }
            ServerMessage::Sync {
                cards,
                current_player,
                turn_player,
                resources,
                health,
                ..
            } => {
                self.data.cards = sort_cards(cards);
                self.current_player = *current_player;
                self.data.current_player = *current_player;
                self.data.turn_player = *turn_player;
                self.data.resources = resources.clone();
                self.data.avatar_health = health.clone();
                self.open_controlled_hand_viewer();
                None
            }
            ServerMessage::ForceSync {
                cards,
                current_player,
                turn_player,
                resources,
                health,
                ..
            } => {
                self.data.cards = sort_cards(cards);
                self.current_player = *current_player;
                self.data.current_player = *current_player;
                self.data.turn_player = *turn_player;
                self.data.resources = resources.clone();
                self.data.avatar_health = health.clone();
                self.open_controlled_hand_viewer();
                None
            }
            _ => None,
        }
    }
}
