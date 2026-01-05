use crate::{
    components::{
        Component, ComponentCommand, ComponentType, event_log::EventLogComponent, player_hand::PlayerHandComponent,
        player_status::PlayerStatusComponent, realm::RealmComponent,
    },
    config::*,
    input::Mouse,
    scene::{Scene, selection_overlay::SelectionOverlay},
    texture_cache::TextureCache,
};
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
    card::{CardType, Plane, RenderableCard, Zone},
    game::{PlayerId, Resources},
    networking::{
        self,
        message::{ClientMessage, ServerMessage},
    },
};
use std::collections::HashMap;

use super::selection_overlay::SelectionOverlayBehaviour;

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
    },
    SelectingPath {
        paths: Vec<Vec<Zone>>,
    },
    SelectingZone {
        zones: Vec<Zone>,
    },
    ViewingCards {
        cards: Vec<uuid::Uuid>,
        prompt: String,
        prev_status: Box<Status>,
        behaviour: SelectionOverlayBehaviour,
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

fn component_rect(component_type: ComponentType) -> Rect {
    match component_type {
        ComponentType::EventLog => event_log_rect(),
        ComponentType::PlayerStatus => Rect::new(20.0, 25.0, realm_rect().x, 60.0),
        ComponentType::PlayerHand => hand_rect(),
        ComponentType::Realm => realm_rect(),
    }
}

#[derive(Debug)]
pub struct GameData {
    pub player_id: PlayerId,
    pub cards: Vec<RenderableCard>,
    pub events: Vec<Event>,
    pub status: Status,
    pub unseen_events: usize,
    pub resources: HashMap<PlayerId, Resources>,
}

impl GameData {
    pub fn new(player_id: &uuid::Uuid, cards: Vec<RenderableCard>) -> Self {
        Self {
            player_id: player_id.clone(),
            cards,
            events: Vec::new(),
            status: Status::Idle,
            unseen_events: 0,
            resources: HashMap::new(),
        }
    }
}

// Takes a slice of cards and returns a cloned, sorted vec with the cards sorted so that cards that
// are submerged or burrowed are first in the vec, then sites, then cards on the surface and then
// cards in the air.
fn sort_cards(cards: &[RenderableCard]) -> Vec<RenderableCard> {
    let mut cards = cards.to_vec();
    cards.sort_by(|a, b| {
        let plane_cmp = a.plane.cmp(&b.plane);
        if plane_cmp != std::cmp::Ordering::Equal {
            return plane_cmp;
        }

        if let Plane::Surface = a.plane {
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

#[derive(Debug)]
pub struct Game {
    opponent_id: PlayerId,
    game_id: uuid::Uuid,
    client: networking::client::Client,
    current_player: PlayerId,
    is_player_one: bool,
    card_selection_overlay: Option<SelectionOverlay>,
    components: Vec<Box<dyn Component>>,
    data: GameData,
}

impl Game {
    pub fn new(
        game_id: uuid::Uuid,
        player_id: uuid::Uuid,
        opponent_id: uuid::Uuid,
        is_player_one: bool,
        cards: Vec<RenderableCard>,
        client: networking::client::Client,
    ) -> Self {
        Self {
            opponent_id,
            game_id: game_id.clone(),
            client: client.clone(),
            current_player: uuid::Uuid::nil(),
            is_player_one,
            card_selection_overlay: None,
            components: vec![
                Box::new(PlayerStatusComponent::new(
                    component_rect(ComponentType::PlayerStatus),
                    opponent_id.clone(),
                    false,
                )),
                Box::new(PlayerStatusComponent::new(
                    component_rect(ComponentType::PlayerStatus),
                    player_id.clone(),
                    true,
                )),
                Box::new(RealmComponent::new(
                    &game_id,
                    &player_id,
                    !is_player_one,
                    client.clone(),
                    component_rect(ComponentType::Realm),
                )),
                Box::new(PlayerHandComponent::new(
                    &game_id,
                    &player_id,
                    client.clone(),
                    component_rect(ComponentType::PlayerHand),
                )),
                Box::new(EventLogComponent::new(component_rect(ComponentType::EventLog))),
            ],
            data: GameData::new(&player_id, cards),
        }
    }

    fn is_players_turn(&self, player_id: &PlayerId) -> bool {
        self.current_player == *player_id
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        for component in &mut self.components {
            component.update(&mut self.data).await?;

            component
                .process_command(&ComponentCommand::SetRect {
                    component_type: component.get_component_type(),
                    rect: component_rect(component.get_component_type()),
                })
                .await;
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            Mouse::record_press();
        }

        if is_mouse_button_released(MouseButton::Left) {
            Mouse::record_release();
            Mouse::set_enabled(true);
        }

        if let Status::ViewingCards {
            cards,
            behaviour,
            prev_status,
            prompt,
        } = &self.data.status
        {
            let renderables = self.data.cards.iter().filter(|c| cards.contains(&c.id)).collect();
            self.card_selection_overlay = Some(
                SelectionOverlay::new(
                    self.client.clone(),
                    &self.game_id,
                    &self.data.player_id,
                    renderables,
                    prompt,
                    behaviour.clone(),
                )
                .await,
            );
            self.data.status = *prev_status.clone();
        }

        if self.card_selection_overlay.is_some() {
            let overlay = self.card_selection_overlay.as_mut().unwrap();
            overlay.update();

            if overlay.should_close() {
                self.card_selection_overlay = None;
                self.data.status = Status::Idle;
            }
        }

        Ok(())
    }

    pub async fn render(&mut self) -> anyhow::Result<()> {
        if self.game_id.is_nil() {
            let time = macroquad::time::get_time();
            let dot_count = ((time * 2.0) as usize % 3) + 1;
            let mut dots = ".".repeat(dot_count);
            dots += &" ".repeat(3 - dot_count);
            let message = format!("Looking for match{}", dots);

            let screen_rect = screen_rect();
            let text_dimensions = macroquad::text::measure_text(&message, None, FONT_SIZE as u16, 1.0);
            let x = screen_rect.w / 2.0 - text_dimensions.width / 2.0;
            let y = screen_rect.h / 2.0 - text_dimensions.height / 2.0;

            draw_text(&message, x, y, 32.0, WHITE);
            return Ok(());
        }

        for component in &mut self.components {
            component.render(&mut self.data).await?;
        }
        self.render_gui().await?;

        if self.card_selection_overlay.is_some() {
            let overlay = self.card_selection_overlay.as_mut().unwrap();
            overlay.render();
        }

        if let Status::Waiting { ref prompt } = self.data.status {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.0, 0.0, 0.0, 0.6),
            );

            let screen_rect = screen_rect();
            let text_dimensions = macroquad::text::measure_text(prompt, None, FONT_SIZE as u16, 1.0);
            let x = screen_rect.w / 2.0 - text_dimensions.width / 2.0;
            let y = screen_rect.h / 2.0 - text_dimensions.height / 2.0;

            draw_text(prompt, x, y, FONT_SIZE, WHITE);
        }

        Ok(())
    }

    pub async fn process_message(&mut self, message: &ServerMessage) -> anyhow::Result<Option<Scene>> {
        match message {
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
            ServerMessage::PickZone { zones, .. } => {
                self.data.status = Status::SelectingZone { zones: zones.clone() };
                Ok(None)
            }
            ServerMessage::PickPath { paths, .. } => {
                self.data.status = Status::SelectingPath { paths: paths.clone() };
                Ok(None)
            }
            ServerMessage::PickCard {
                cards, prompt, preview, ..
            } => {
                self.data.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: preview.clone(),
                    prompt: prompt.clone(),
                };

                if *preview {
                    let renderables = self.data.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                    self.card_selection_overlay = Some(
                        SelectionOverlay::new(
                            self.client.clone(),
                            &self.game_id,
                            &self.data.player_id,
                            renderables,
                            prompt,
                            SelectionOverlayBehaviour::Pick,
                        )
                        .await,
                    );
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
            ServerMessage::GameStarted {
                game_id,
                player1,
                player2,
                cards,
                ..
            } => {
                self.is_player_one = player1 == &self.data.player_id;
                self.game_id = game_id.clone();
                self.opponent_id = if self.is_player_one {
                    player2.clone()
                } else {
                    player1.clone()
                };
                TextureCache::load_cache(cards).await;
                Ok(None)
            }
            ServerMessage::Sync {
                cards,
                current_player,
                resources,
            } => {
                self.data.cards = sort_cards(cards);
                self.current_player = current_player.clone();
                self.data.resources = resources.clone();
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

    pub async fn process_input(&mut self) {
        if let Status::Waiting { .. } = self.data.status {
            return;
        }

        if self.card_selection_overlay.is_some() {
            let overlay = self.card_selection_overlay.as_mut().unwrap();
            overlay.process_input();
            return;
        }

        let mut component_actions = vec![];
        for component in &mut self.components {
            if let Ok(Some(action)) =
                component.process_input(self.current_player == self.data.player_id, &mut self.data)
            {
                component_actions.push(action);
            }
        }

        for action in component_actions {
            for component in &mut self.components {
                component.process_command(&action).await;
            }
        }
    }

    async fn render_gui(&mut self) -> anyhow::Result<()> {
        let screen_rect = screen_rect();
        let turn_label = if self.is_players_turn(&self.data.player_id) {
            "Your Turn"
        } else {
            "Opponent's Turn"
        };

        draw_text(turn_label, screen_rect.w / 2.0 - 50.0, 30.0, FONT_SIZE, WHITE);

        let is_in_turn = self.current_player == self.data.player_id;
        let is_idle = matches!(self.data.status, Status::Idle);
        if is_in_turn && is_idle {
            if ui::root_ui().button(Vec2::new(screen_rect.w - 100.0, screen_rect.h - 40.0), "Pass Turn") {
                Mouse::set_enabled(false);
                self.client.send(ClientMessage::EndTurn {
                    player_id: self.data.player_id.clone(),
                    game_id: self.game_id.clone(),
                })?;
            }
        }

        match self.data.status {
            Status::SelectingAction {
                ref prompt,
                ref actions,
            } => {
                // Draw semi-transparent overlay behind the action selection window
                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::new(0.0, 0.0, 0.0, 0.6),
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

                let prompt = prompt.clone();
                let button_height = 30.0;
                let window_size = Vec2::new(400.0, (button_height + 10.0) * actions.len() as f32 + 20.0 + 50.0);
                let actions = actions.clone();
                ui::root_ui().window(
                    hash!(),
                    Vec2::new(
                        screen_width() / 2.0 - window_size.x / 2.0,
                        screen_height() / 2.0 - window_size.y / 2.0,
                    ),
                    window_size,
                    |ui| {
                        ui::widgets::Label::new(&prompt)
                            .position(Vec2::new(5.0, 5.0))
                            .multiline(10.0)
                            .ui(ui);
                        for (idx, action) in actions.iter().enumerate() {
                            let button_pos =
                                Vec2::new(window_size.x * 0.1, (button_height + 10.0) * (idx as f32 + 1.0));
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
                                    .unwrap();
                                Mouse::set_enabled(false);
                                self.data.status = Status::Idle;
                            }
                        }
                    },
                );

                ui::root_ui().pop_skin();
            }
            _ => {}
        }

        Ok(())
    }

    pub fn wrap_text<S: AsRef<str>>(text: S, max_width: f32, font_size: u16) -> String {
        use macroquad::text::measure_text;
        let mut lines = Vec::new();
        for paragraph in text.as_ref().split('\n') {
            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                let test = if current.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current, word)
                };
                let dims = measure_text(&test, None, font_size, 1.0);
                if dims.width > max_width && !current.is_empty() {
                    lines.push(current.clone());
                    current = word.to_string();
                } else {
                    current = test;
                }
            }
            if !current.is_empty() {
                lines.push(current);
            }
        }
        lines.join("\n")
    }
}
