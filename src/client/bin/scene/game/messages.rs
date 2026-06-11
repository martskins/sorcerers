use super::*;

fn retain_aura_affected_zones(
    cache: &mut HashMap<uuid::Uuid, Option<Vec<Location>>>,
    previous_cards: &[CardData],
    next_cards: &[CardData],
) {
    cache.retain(|card_id, _| {
        let previous = previous_cards.iter().find(|card| card.id == *card_id);
        let next = next_cards.iter().find(|card| card.id == *card_id);

        matches!(
            (previous, next),
            (Some(previous), Some(next))
                if next.card_type == CardType::Aura && previous.zone == next.zone
        )
    });
}

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
            ServerMessage::ProjectileFired {
                shooter,
                origin,
                direction,
                range,
                ranged_strike,
                ..
            } => {
                self.data
                    .pending_projectiles
                    .push(PendingProjectileAnimation {
                        id: uuid::Uuid::new_v4(),
                        shooter: *shooter,
                        origin: origin.clone(),
                        direction: direction.clone(),
                        range: *range,
                        ranged_strike: *ranged_strike,
                    });
                None
            }
            ServerMessage::PlayerDisconnected { player_id, .. } => {
                self.data.status = Status::GameAborted {
                    reason: format!("Player {} disconnected.", player_id),
                };
                None
            }
            ServerMessage::GameOver {
                winner_id,
                winner_name,
                ..
            } => {
                self.data.status = Status::GameOver {
                    winner_id: *winner_id,
                    winner_name: winner_name.clone(),
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
            ServerMessage::PickLocationGroup {
                groups,
                prompt,
                source_card_id,
                ..
            } => {
                self.data.status = Status::SelectingZoneGroup {
                    groups: groups.clone(),
                    prompt: prompt.clone(),
                    source_card_id: *source_card_id,
                };
                None
            }
            ServerMessage::PickLocation {
                locations,
                prompt,
                source_card_id,
                ..
            } => {
                self.data.status = Status::SelectingZone {
                    locations: locations.clone(),
                    prompt: prompt.clone(),
                    source_card_id: *source_card_id,
                };
                None
            }
            ServerMessage::PlayableLocations {
                card_id, locations, ..
            } => {
                self.data.status = Status::PreviewingPlayableLocations {
                    card_id: *card_id,
                    locations: locations.clone(),
                };
                None
            }
            ServerMessage::AuraAreOfEffect {
                card_id, locations, ..
            } => {
                self.data
                    .aura_areas_of_effect
                    .insert(*card_id, Some(locations.clone()));
                None
            }
            ServerMessage::OngoingEffects { effects, .. } => {
                self.data.ongoing_effects = Some(effects.clone());
                None
            }
            ServerMessage::PickAmount {
                prompt,
                source_card_id,
                min_amount,
                max_amount,
                ..
            } => {
                self.data.status = Status::SelectingAmount {
                    min_amount: *min_amount,
                    max_amount: *max_amount,
                    prompt: prompt.clone(),
                    source_card_id: *source_card_id,
                };
                None
            }
            ServerMessage::PickPath {
                paths,
                prompt,
                source_card_id,
                ..
            } => {
                self.data.status = Status::SelectingPath {
                    paths: paths.clone(),
                    prompt: prompt.clone(),
                    source_card_id: *source_card_id,
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
                self.overlay = Some(GameOverlay::Action(ActionOverlay::new(
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
                source_card_id,
                ..
            } => {
                self.data.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: *preview,
                    prompt: prompt.clone(),
                    source_card_id: *source_card_id,
                    multiple: true,
                };
                if *preview {
                    let renderables = self
                        .data
                        .cards
                        .iter()
                        .filter(|c| cards.contains(&c.id))
                        .collect();
                    self.overlay = Some(GameOverlay::Selection(SelectionOverlay::new(
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
                    self.overlay =
                        Some(GameOverlay::CombatResolution(CombatResolutionOverlay::new(
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
                source_card_id,
                ..
            } => {
                self.data.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: *preview,
                    prompt: prompt.clone(),
                    source_card_id: *source_card_id,
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
                    self.overlay = Some(GameOverlay::Selection(SelectionOverlay::new(
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
                source_card_id,
                anchor_on_cursor,
                ..
            } => {
                self.data.status = Status::SelectingAction {
                    prompt: prompt.to_string(),
                    actions: actions.clone(),
                    source_card_id: *source_card_id,
                    anchor_on_cursor: *anchor_on_cursor,
                };
                None
            }
            ServerMessage::PickDirection {
                prompt,
                directions,
                source_card_id,
                ..
            } => {
                self.data.status = Status::SelectingDirection {
                    prompt: prompt.to_string(),
                    directions: directions.clone(),
                    source_card_id: *source_card_id,
                };
                None
            }
            ServerMessage::Sync {
                cards,
                current_player,
                turn_player,
                resources,
                health,
                stepped_effects,
                effect_queue,
                ..
            } => {
                retain_aura_affected_zones(
                    &mut self.data.aura_areas_of_effect,
                    &self.data.cards,
                    cards,
                );
                self.data.cards = sort_cards(cards);
                self.current_player = *current_player;
                self.data.current_player = *current_player;
                self.data.turn_player = *turn_player;
                self.data.resources = resources.clone();
                self.data.avatar_health = health.clone();
                self.data.ongoing_effects = None;
                self.data.highlighted_ongoing_effect = None;
                self.data.stepped_effects = *stepped_effects;
                self.data.effect_queue = effect_queue.clone();
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
                retain_aura_affected_zones(
                    &mut self.data.aura_areas_of_effect,
                    &self.data.cards,
                    cards,
                );
                self.data.cards = sort_cards(cards);
                self.current_player = *current_player;
                self.data.current_player = *current_player;
                self.data.turn_player = *turn_player;
                self.data.resources = resources.clone();
                self.data.avatar_health = health.clone();
                self.data.ongoing_effects = None;
                self.data.highlighted_ongoing_effect = None;
                self.open_controlled_hand_viewer();
                None
            }
            _ => None,
        }
    }
}
