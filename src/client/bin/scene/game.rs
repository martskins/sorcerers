use super::selection_overlay::SelectionOverlayBehaviour;
use crate::{
    components::{
        Component, ComponentCommand, ComponentType, event_log::EventLogComponent, player_hand::PlayerHandComponent,
        player_status::PlayerStatusComponent, realm::RealmComponent,
    },
    config::*,
    input::Mouse,
    render,
    scene::{
        Scene, action_overlay::ActionOverlay, combat_resolution_overlay::CombatResolutionOverlay, menu::Menu,
        selection_overlay::SelectionOverlay,
    },
};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData};
use macroquad::{
    color::{Color, WHITE},
    input::{MouseButton, is_mouse_button_pressed, is_mouse_button_released},
    math::{Rect, RectOffset, Vec2},
    shapes::draw_rectangle,
    text::draw_text,
    ui::{self, hash},
    window::{screen_height, screen_width},
};
use sorcerers::{
    card::{CardData, CardType, Region, Zone},
    game::{PlayerId, Resources},
    networking::{
        self,
        message::{ClientMessage, ServerMessage},
    },
};
use std::collections::HashMap;

const FONT_SIZE: f32 = 24.0;

#[derive(Debug, PartialEq, Clone)]
pub enum Status {
    Idle,
    Waiting {
        prompt: String,
    },
    SelectingAction {
        actions: Vec<String>,
        prompt: String,
    },
    SelectingCard {
        cards: Vec<uuid::Uuid>,
        preview: bool,
        prompt: String,
        multiple: bool,
    },
    SelectingPath {
        paths: Vec<Vec<Zone>>,
        prompt: String,
    },
    SelectingZoneGroup {
        groups: Vec<Vec<Zone>>,
        prompt: String,
    },
    SelectingZone {
        zones: Vec<Zone>,
        prompt: String,
    },
    ViewingCards {
        cards: Vec<uuid::Uuid>,
        prompt: String,
        prev_status: Box<Status>,
        behaviour: SelectionOverlayBehaviour,
    },
    DistributingDamage {
        player_id: PlayerId,
        attacker: uuid::Uuid,
        defenders: Vec<uuid::Uuid>,
        damage: u16,
    },
    GameAborted {
        reason: String,
    },
}

#[derive(Debug)]
pub struct Event {
    pub id: uuid::Uuid,
    pub description: String,
    pub datetime: chrono::DateTime<chrono::Utc>,
}

impl Event {
    fn formatted_datetime(&self) -> String {
        self.datetime.format("%H:%M:%S").to_string()
    }

    pub fn formatted(&self) -> String {
        format!("{}: {}", self.formatted_datetime(), self.description)
    }
}

fn component_rect(component_type: ComponentType) -> anyhow::Result<Rect> {
    match component_type {
        ComponentType::EventLog => Ok(event_log_rect()),
        ComponentType::PlayerStatus => Ok(Rect::new(20.0, 25.0, realm_rect()?.x, 60.0)),
        ComponentType::PlayerHand => hand_rect(),
        ComponentType::Realm => realm_rect(),
        ComponentType::SelectionOverlay => screen_rect(),
        ComponentType::CombatResolutionOverlay => screen_rect(),
        ComponentType::ActionOverlay => screen_rect(),
    }
}

#[derive(Debug)]
pub struct GameData {
    pub player_id: PlayerId,
    pub cards: Vec<CardData>,
    pub events: Vec<Event>,
    pub status: Status,
    pub unseen_events: usize,
    pub resources: HashMap<PlayerId, Resources>,
    pub avatar_health: HashMap<PlayerId, u16>,
}

impl GameData {
    pub fn new(player_id: &uuid::Uuid, cards: Vec<CardData>) -> Self {
        Self {
            player_id: player_id.clone(),
            cards,
            events: Vec::new(),
            status: Status::Idle,
            unseen_events: 0,
            resources: HashMap::new(),
            avatar_health: HashMap::new(),
        }
    }
}

// Takes a slice of cards and returns a cloned, sorted vec with the cards sorted so that cards that
// are submerged or burrowed are first in the vec, then sites, then cards on the surface and then
// cards in the air.
fn sort_cards(cards: &[CardData]) -> Vec<CardData> {
    let mut cards = cards.to_vec();
    cards.sort_by(|a, b| {
        let region_cmp = a.region.cmp(&b.region);
        if region_cmp != std::cmp::Ordering::Equal {
            return region_cmp;
        }

        if let Region::Surface = a.region {
            match (&a.card_type, &b.card_type) {
                (CardType::Site, CardType::Site) => std::cmp::Ordering::Equal,
                (CardType::Site, _) => std::cmp::Ordering::Less,
                (_, CardType::Site) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        } else {
            std::cmp::Ordering::Equal
        }
    });

    cards
}

pub struct Game {
    game_id: uuid::Uuid,
    client: networking::client::Client,
    current_player: PlayerId,
    overlay: Option<Box<dyn Component>>,
    components: Vec<Box<dyn Component>>,
    data: GameData,
    audio_manager: AudioManager<DefaultBackend>,
}

impl Game {
    pub fn new(
        game_id: uuid::Uuid,
        player_id: uuid::Uuid,
        opponent_id: uuid::Uuid,
        is_player_one: bool,
        cards: Vec<CardData>,
        client: networking::client::Client,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            game_id: game_id.clone(),
            client: client.clone(),
            current_player: uuid::Uuid::nil(),
            overlay: None,
            components: vec![
                Box::new(PlayerStatusComponent::new(
                    component_rect(ComponentType::PlayerStatus)?,
                    opponent_id.clone(),
                    false,
                )),
                Box::new(PlayerStatusComponent::new(
                    component_rect(ComponentType::PlayerStatus)?,
                    player_id.clone(),
                    true,
                )),
                Box::new(RealmComponent::new(
                    &game_id,
                    &player_id,
                    !is_player_one,
                    client.clone(),
                    component_rect(ComponentType::Realm)?,
                )),
                Box::new(PlayerHandComponent::new(
                    &game_id,
                    &player_id,
                    client.clone(),
                    component_rect(ComponentType::PlayerHand)?,
                )),
                Box::new(EventLogComponent::new(component_rect(ComponentType::EventLog)?)),
            ],
            data: GameData::new(&player_id, cards),
            audio_manager: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
                .expect("AudioManager to be created"),
        })
    }

    fn is_players_turn(&self, player_id: &PlayerId) -> bool {
        self.current_player == *player_id
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        for component in &mut self.components {
            component.update(&mut self.data).await?;

            component
                .process_command(
                    &ComponentCommand::SetRect {
                        component_type: component.get_component_type(),
                        rect: component_rect(component.get_component_type())?,
                    },
                    &mut self.data,
                )
                .await?;
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            Mouse::record_press()?;
        }

        if is_mouse_button_released(MouseButton::Left) {
            Mouse::record_release()?;
            Mouse::set_enabled(true)?;
        }

        if let Status::ViewingCards {
            cards,
            behaviour,
            prev_status,
            prompt,
        } = &self.data.status
        {
            let renderables = self.data.cards.iter().filter(|c| cards.contains(&c.id)).collect();
            self.overlay = Some(Box::new(
                SelectionOverlay::new(
                    self.client.clone(),
                    &self.game_id,
                    &self.data.player_id,
                    renderables,
                    prompt,
                    behaviour.clone(),
                )
                .await?,
            ));
            self.data.status = *prev_status.clone();
        }

        if let Some(overlay) = &mut self.overlay {
            overlay.update(&mut self.data).await?;
        }

        Ok(())
    }

    pub async fn render(&mut self) -> anyhow::Result<Option<Scene>> {
        if self.game_id.is_nil() {
            let time = macroquad::time::get_time();
            let dot_count = ((time * 2.0) as usize % 3) + 1;
            let mut dots = ".".repeat(dot_count);
            dots += &" ".repeat(3 - dot_count);
            let message = format!("Looking for match{}", dots);

            let screen_rect = screen_rect()?;
            let text_dimensions = macroquad::text::measure_text(&message, None, FONT_SIZE as u16, 1.0);
            let x = screen_rect.w / 2.0 - text_dimensions.width / 2.0;
            let y = screen_rect.h / 2.0 - text_dimensions.height / 2.0;

            draw_text(&message, x, y, 32.0, WHITE);
            return Ok(None);
        }

        for component in &mut self.components {
            component.render(&mut self.data).await?;
        }

        if let Some(scene) = self.render_gui().await? {
            return Ok(Some(scene));
        }

        if let Some(overlay) = &mut self.overlay {
            overlay.render(&mut self.data).await?;
        }

        Ok(None)
    }

    pub async fn process_message(&mut self, message: &ServerMessage) -> anyhow::Result<Option<Scene>> {
        match message {
            ServerMessage::PlaySoundEffect { .. } => {
                let sound_data = StaticSoundData::from_file("assets/sounds/play_card.mp3")?;
                self.audio_manager.play(sound_data.clone())?;
                Ok(None)
            }
            ServerMessage::PlayerDisconnected { player_id, .. } => {
                self.data.status = Status::GameAborted {
                    reason: format!("Player {} disconnected.", player_id),
                };
                Ok(None)
            }
            ServerMessage::Resume { .. } => {
                self.data.status = Status::Idle;
                Ok(None)
            }
            ServerMessage::Wait { prompt, .. } => {
                self.data.status = Status::Waiting { prompt: prompt.clone() };
                Ok(None)
            }
            ServerMessage::LogEvent {
                id,
                description,
                datetime,
            } => {
                self.data.events.push(Event {
                    id: id.clone(),
                    description: description.clone(),
                    datetime: datetime.clone(),
                });
                Ok(None)
            }
            ServerMessage::PickZoneGroup {
                groups: zones, prompt, ..
            } => {
                self.data.status = Status::SelectingZoneGroup {
                    groups: zones.clone(),
                    prompt: prompt.clone(),
                };
                Ok(None)
            }
            ServerMessage::PickZone { zones, prompt, .. } => {
                self.data.status = Status::SelectingZone {
                    zones: zones.clone(),
                    prompt: prompt.clone(),
                };
                Ok(None)
            }
            ServerMessage::PickPath { paths, prompt, .. } => {
                self.data.status = Status::SelectingPath {
                    paths: paths.clone(),
                    prompt: prompt.clone(),
                };
                Ok(None)
            }
            ServerMessage::RevealCards {
                cards, action, prompt, ..
            } => {
                let renderables = self.data.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                self.overlay = Some(Box::new(
                    ActionOverlay::new(
                        self.client.clone(),
                        &self.game_id,
                        renderables,
                        &self.data.player_id,
                        prompt.to_string(),
                        action.clone(),
                    )
                    .await?,
                ));
                Ok(None)
            }

            ServerMessage::PickCards {
                cards, prompt, preview, ..
            } => {
                self.data.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: preview.clone(),
                    prompt: prompt.clone(),
                    multiple: true,
                };

                if *preview {
                    let renderables = self.data.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                    self.overlay = Some(Box::new(
                        SelectionOverlay::new(
                            self.client.clone(),
                            &self.game_id,
                            &self.data.player_id,
                            renderables,
                            prompt,
                            SelectionOverlayBehaviour::Pick,
                        )
                        .await?,
                    ));
                }
                Ok(None)
            }
            ServerMessage::DistributeDamage {
                player_id,
                attacker,
                defenders,
                damage,
            } => {
                self.data.status = Status::DistributingDamage {
                    player_id: player_id.clone(),
                    attacker: attacker.clone(),
                    defenders: defenders.clone(),
                    damage: *damage,
                };

                let defenders = self
                    .data
                    .cards
                    .iter()
                    .filter(|c| defenders.contains(&c.id))
                    .cloned()
                    .collect::<Vec<CardData>>();
                let attacker = self
                    .data
                    .cards
                    .iter()
                    .find(|c| c.id == *attacker)
                    .ok_or(anyhow::anyhow!("Attacker card not found"))?
                    .clone();
                self.overlay = Some(Box::new(
                    CombatResolutionOverlay::new(
                        self.client.clone(),
                        &self.game_id,
                        player_id,
                        attacker,
                        defenders,
                        damage.clone(),
                    )
                    .await?,
                ));

                Ok(None)
            }
            ServerMessage::PickCard {
                cards, prompt, preview, ..
            } => {
                self.data.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: preview.clone(),
                    prompt: prompt.clone(),
                    multiple: false,
                };

                if *preview {
                    let renderables = self.data.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                    self.overlay = Some(Box::new(
                        SelectionOverlay::new(
                            self.client.clone(),
                            &self.game_id,
                            &self.data.player_id,
                            renderables,
                            prompt,
                            SelectionOverlayBehaviour::Pick,
                        )
                        .await?,
                    ));
                }
                Ok(None)
            }
            ServerMessage::PickAction { prompt, actions, .. } => {
                self.data.status = Status::SelectingAction {
                    prompt: prompt.to_string(),
                    actions: actions.clone(),
                };
                Ok(None)
            }
            ServerMessage::Sync {
                cards,
                current_player,
                resources,
                health,
            } => {
                self.data.cards = sort_cards(cards);
                self.current_player = current_player.clone();
                self.data.resources = resources.clone();
                self.data.avatar_health = health.clone();
                Ok(None)
            }
            ServerMessage::ForceSync {
                cards,
                current_player,
                resources,
                ..
            } => {
                self.data.cards = sort_cards(cards);
                self.current_player = current_player.clone();
                self.data.resources = resources.clone();
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    pub async fn process_input(&mut self) -> anyhow::Result<()> {
        if let Status::Waiting { .. } = self.data.status {
            return Ok(());
        }

        if let Some(overlay) = &mut self.overlay {
            overlay
                .process_input(self.current_player == self.data.player_id, &mut self.data)
                .await?;

            if !overlay.is_visible() {
                self.overlay = None;
            }
        }

        let mut component_actions = vec![];
        for component in &mut self.components {
            if let Ok(Some(action)) = component
                .process_input(self.current_player == self.data.player_id, &mut self.data)
                .await
            {
                component_actions.push(action);
            }
        }

        for action in component_actions {
            for component in &mut self.components {
                component.process_command(&action, &mut self.data).await?;
            }
        }

        Ok(())
    }

    async fn render_gui(&mut self) -> anyhow::Result<Option<Scene>> {
        let screen_rect = screen_rect()?;
        let turn_label = if self.is_players_turn(&self.data.player_id) {
            "Your Turn"
        } else {
            "Their Turn"
        };

        draw_text(turn_label, 10.0, 120.0, FONT_SIZE, WHITE);

        let is_in_turn = self.current_player == self.data.player_id;
        let is_idle = matches!(self.data.status, Status::Idle);
        if is_in_turn && is_idle {
            if ui::root_ui().button(Vec2::new(screen_rect.w - 100.0, screen_rect.h - 40.0), "Pass Turn") {
                Mouse::set_enabled(false)?;
                self.client.send(ClientMessage::EndTurn {
                    player_id: self.data.player_id.clone(),
                    game_id: self.game_id.clone(),
                })?;
            }
        } else if matches!(self.data.status, Status::SelectingCard { multiple: true, .. }) {
            if ui::root_ui().button(Vec2::new(screen_rect.w - 120.0, screen_rect.h - 40.0), "Done Selecting") {
                Mouse::set_enabled(false)?;
                for component in &mut self.components {
                    component
                        .process_command(&ComponentCommand::DonePicking, &mut self.data)
                        .await?;
                }
            }
        }

        let needs_overlay = matches!(
            &self.data.status,
            Status::Waiting { .. } | Status::SelectingAction { .. } | Status::GameAborted { .. }
        );
        if needs_overlay {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.0, 0.0, 0.0, 0.8),
            );

            let window_style = ui::root_ui()
                .style_builder()
                .background_margin(RectOffset::new(10.0, 10.0, 10.0, 10.0))
                .build();
            let skin = ui::Skin {
                window_style,
                ..ui::root_ui().default_skin()
            };

            ui::root_ui().push_skin(&skin);
        }

        let new_scene = match self.data.status {
            Status::Waiting { ref prompt } => {
                let text_dimensions = macroquad::text::measure_text(prompt, None, FONT_SIZE as u16, 1.0);
                let x = screen_rect.w / 2.0 - text_dimensions.width / 2.0;
                let y = screen_rect.h / 2.0 - text_dimensions.height / 2.0;

                draw_text(prompt, x, y, FONT_SIZE, WHITE);
                None
            }
            Status::SelectingAction {
                ref prompt,
                ref actions,
            } => {
                let button_height = 30.0;
                let window_width = 400.0;
                let font_size = 16;
                let prompt = render::wrap_text(prompt, window_width - 10.0, font_size);
                let label_padding = 10.0;
                let label_height = prompt.lines().count() as f32 * (font_size as f32 + 4.0) + label_padding;
                let window_size = Vec2::new(
                    window_width,
                    (button_height + 10.0) * actions.len() as f32 + 20.0 + 50.0,
                );
                let actions = actions.clone();
                let mut disable_mouse = false;
                ui::root_ui().window(
                    hash!(),
                    Vec2::new(
                        screen_width() / 2.0 - window_size.x / 2.0,
                        screen_height() / 2.0 - window_size.y / 2.0,
                    ),
                    window_size,
                    |ui| {
                        render::multiline_label(&prompt, Vec2::new(5.0, 5.0), font_size, ui);

                        for (idx, action) in actions.iter().enumerate() {
                            let button_pos =
                                Vec2::new(window_size.x * 0.1, (button_height + 10.0) * idx as f32 + label_height);
                            let clicked = ui::widgets::Button::new(action.as_str())
                                .position(button_pos)
                                .size(Vec2::new(window_size.x * 0.8, button_height))
                                .ui(ui);
                            if clicked {
                                self.client
                                    .send(ClientMessage::PickAction {
                                        game_id: self.game_id,
                                        player_id: self.data.player_id,
                                        action_idx: idx,
                                    })
                                    .expect("PickAction to be sent");
                                disable_mouse = true;
                                self.data.status = Status::Idle;
                            }
                        }
                    },
                );

                if disable_mouse {
                    Mouse::set_enabled(false)?;
                }

                None
            }
            Status::GameAborted { ref reason } => {
                let mut scene = None;
                let button_height = 30.0;
                let window_width = 400.0;
                let font_size = 16;
                let prompt = render::wrap_text(reason, window_width - 10.0, font_size);
                let label_padding = 10.0;
                let label_height = prompt.lines().count() as f32 * (font_size as f32 + 4.0) + label_padding;
                let window_size = Vec2::new(window_width, button_height + 10.0 + 20.0 + label_height);
                let mut disable_mouse = false;
                ui::root_ui().window(
                    hash!(),
                    Vec2::new(
                        screen_width() / 2.0 - window_size.x / 2.0,
                        screen_height() / 2.0 - window_size.y / 2.0,
                    ),
                    window_size,
                    |ui| {
                        render::multiline_label(&prompt, Vec2::new(5.0, 5.0), font_size, ui);
                        let button_pos = Vec2::new(window_size.x * 0.1, label_height);
                        let clicked = ui::widgets::Button::new("Ok")
                            .position(button_pos)
                            .size(Vec2::new(window_size.x * 0.8, button_height))
                            .ui(ui);
                        if clicked {
                            scene = Some(Scene::Menu(Menu::new(self.client.clone())));
                            disable_mouse = true;
                            self.data.status = Status::Idle;
                        }
                    },
                );

                if disable_mouse {
                    Mouse::set_enabled(false)?;
                }

                scene
            }
            _ => None,
        };

        ui::root_ui().pop_skin();

        Ok(new_scene)
    }
}
