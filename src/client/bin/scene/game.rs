use super::selection_overlay::SelectionOverlayBehaviour;
use crate::{
    components::{
        Component, ComponentCommand, ComponentType, card_viewer::CardViewerComponent,
        event_log::EventLogComponent, player_hand::PlayerHandComponent,
        player_status::PlayerStatusComponent, realm::RealmComponent,
    },
    config::*,
    render::popup_action_menu,
    scene::{
        Scene,
        action_overlay::ActionOverlay,
        card_toast::{CardToast, TOAST_MARGIN},
        combat_resolution_overlay::CombatResolutionOverlay,
        menu::Menu,
        selection_overlay::SelectionOverlay,
    },
};
use egui::{Color32, Context, FontId, Painter, Rect, RichText, Ui, pos2, vec2};
use kira::{AudioManager, DefaultBackend, sound::static_sound::StaticSoundData};
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
    Mulligan,
    Waiting {
        prompt: String,
    },
    SelectingAction {
        actions: Vec<String>,
        prompt: String,
        anchor_on_cursor: bool,
    },
    SelectingCard {
        cards: Vec<uuid::Uuid>,
        preview: bool,
        prompt: String,
        multiple: bool,
    },
    SelectingAmount {
        prompt: String,
        min_amount: u8,
        max_amount: u8,
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
        ComponentType::PlayerStatus => Ok(Rect::from_min_size(
            pos2(20.0, 25.0),
            vec2(realm_rect()?.min.x, 60.0),
        )),
        ComponentType::PlayerHand => hand_rect(),
        ComponentType::Realm => realm_rect(),
        ComponentType::SelectionOverlay => screen_rect(),
        ComponentType::CombatResolutionOverlay => screen_rect(),
        ComponentType::ActionOverlay => screen_rect(),
        ComponentType::CardViewer => screen_rect(),
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
    /// Screen position of the last card the player clicked; used to anchor context menus.
    pub last_clicked_card_pos: Option<egui::Pos2>,
    pub last_clicked_card_id: Option<uuid::Uuid>,
    pub last_clicked_card_time: Option<f64>,
}

impl GameData {
    pub fn new(player_id: &PlayerId, cards: Vec<CardData>) -> Self {
        Self {
            player_id: *player_id,
            cards,
            events: Vec::new(),
            status: Status::Mulligan,
            unseen_events: 0,
            resources: HashMap::new(),
            avatar_health: HashMap::new(),
            last_clicked_card_pos: None,
            last_clicked_card_id: None,
            last_clicked_card_time: None,
        }
    }
}

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
    player_id: PlayerId,
    opponent_id: PlayerId,
    client: networking::client::Client,
    current_player: PlayerId,
    overlay: Option<Box<dyn Component>>,
    components: Vec<Box<dyn Component>>,
    data: GameData,
    audio_manager: AudioManager<DefaultBackend>,
    selected_value: Option<Box<dyn std::any::Any>>,
    card_toast: Vec<CardToast>,
}

impl Game {
    pub fn new(
        game_id: uuid::Uuid,
        player_id: PlayerId,
        opponent_id: uuid::Uuid,
        is_player_one: bool,
        cards: Vec<CardData>,
        client: networking::client::Client,
        audio_manager: AudioManager<DefaultBackend>,
    ) -> Self {
        let player_status_rect = component_rect(ComponentType::PlayerStatus).unwrap_or(Rect::ZERO);
        let realm_r = component_rect(ComponentType::Realm).unwrap_or(Rect::ZERO);
        let hand_r = component_rect(ComponentType::PlayerHand).unwrap_or(Rect::ZERO);
        let log_rect = event_log_rect();

        Self {
            game_id,
            player_id,
            opponent_id,
            client: client.clone(),
            current_player: uuid::Uuid::nil(),
            overlay: None,
            components: vec![
                Box::new(PlayerStatusComponent::new(
                    player_status_rect,
                    opponent_id,
                    false,
                )),
                Box::new(PlayerStatusComponent::new(
                    player_status_rect,
                    player_id,
                    true,
                )),
                Box::new(RealmComponent::new(
                    &game_id,
                    &player_id,
                    !is_player_one,
                    client.clone(),
                    realm_r,
                )),
                Box::new(PlayerHandComponent::new(
                    &game_id,
                    &player_id,
                    client.clone(),
                    hand_r,
                )),
                Box::new(EventLogComponent::new(log_rect)),
                Box::new(CardViewerComponent::new(
                    &game_id,
                    &player_id,
                    client.clone(),
                )),
            ],
            data: GameData::new(&player_id, cards),
            audio_manager,
            selected_value: None,
            card_toast: Vec::new(),
        }
    }

    /// Push a toast to the queue, dropping the oldest if the cap is exceeded.
    fn push_toast(&mut self, toast: CardToast) {
        const MAX_TOASTS: usize = 8;
        if self.card_toast.len() >= MAX_TOASTS {
            self.card_toast.remove(0);
        }
        self.card_toast.push(toast);
    }

    pub fn update(&mut self, ctx: &Context) {
        for component in &mut self.components {
            if let Err(e) = component.update(&mut self.data, ctx) {
                eprintln!("Error updating component: {}", e);
            }
            if let Ok(rect) = component_rect(component.get_component_type()) {
                let _ = component.process_command(
                    &ComponentCommand::SetRect {
                        component_type: component.get_component_type(),
                        rect,
                    },
                    &mut self.data,
                );
            }
        }

        if let Status::ViewingCards {
            cards,
            behaviour,
            prev_status,
            prompt,
        } = &self.data.status.clone()
        {
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
                behaviour.clone(),
            )));
            self.data.status = *prev_status.clone();
        }

        if let Some(overlay) = &mut self.overlay
            && let Err(e) = overlay.update(&mut self.data, ctx) {
                eprintln!("Error updating overlay: {}", e);
            }
    }

    pub fn render(&mut self, ui: &mut Ui) -> Option<Scene> {
        let painter = ui.painter().clone();

        if self.game_id.is_nil() {
            let time = ui.ctx().input(|i| i.time);
            let dot_count = ((time * 2.0) as usize % 3) + 1;
            let dots = ".".repeat(dot_count) + &" ".repeat(3 - dot_count);
            let message = format!("Looking for match{}", dots);
            let sr = screen_rect().unwrap_or(Rect::ZERO);
            painter.text(
                sr.center(),
                egui::Align2::CENTER_CENTER,
                &message,
                FontId::proportional(32.0),
                Color32::WHITE,
            );
            return None;
        }

        let mut component_actions: Vec<ComponentCommand> = Vec::new();
        for component in &mut self.components {
            let actions = component.render(&mut self.data, ui, &painter).unwrap();
            if let Some(action) = actions {
                component_actions.push(action);
            }
        }

        for action in component_actions {
            for component in &mut self.components {
                let _ = component.process_command(&action, &mut self.data);
            }
        }
        let new_scene = self.render_gui(ui, &painter);

        // Toasts — drawn above the board but below any blocking overlay.
        // Stack from the bottom of the realm area upward (oldest at bottom).
        {
            const TOAST_GAP: f32 = 4.0;
            let realm_bottom = realm_rect().unwrap_or(Rect::ZERO).max.y;
            let mut bottom_y = realm_bottom - TOAST_MARGIN;
            let mut expired: Vec<usize> = Vec::new();
            for (i, toast) in self.card_toast.iter_mut().enumerate() {
                bottom_y -= toast.height(ui.ctx());
                if !toast.render(ui.ctx(), ui, bottom_y) {
                    expired.push(i);
                }
                bottom_y -= TOAST_GAP;
            }
            for i in expired.into_iter().rev() {
                self.card_toast.remove(i);
            }
        }

        if let Some(overlay) = &mut self.overlay
            && let Err(e) = overlay.render(&mut self.data, ui, &painter) {
                eprintln!("Error rendering overlay: {}", e);
            }

        new_scene
    }

    fn open_viewers(&mut self, cards: &Vec<uuid::Uuid>) -> anyhow::Result<()> {
        let open_opponent_cemetery = self
            .data
            .cards
            .iter()
            .filter(|c| c.owner_id == self.opponent_id)
            .any(|c| cards.contains(&c.id) && c.zone == Zone::Cemetery);
        let open_player_cemetery = self
            .data
            .cards
            .iter()
            .filter(|c| c.owner_id == self.player_id)
            .any(|c| cards.contains(&c.id) && c.zone == Zone::Cemetery);

        if open_player_cemetery {
            let command = ComponentCommand::OpenCardViewer {
                title: "Your Cemetery".to_string(),
                zone: Zone::Cemetery,
                controller_id: Some(self.player_id),
            };
            for cmp in self.components.iter_mut() {
                cmp.process_command(&command, &mut self.data).unwrap();
            }
        }

        if open_opponent_cemetery {
            let command = ComponentCommand::OpenCardViewer {
                title: "Opponent's Cemetery".to_string(),
                zone: Zone::Cemetery,
                controller_id: Some(self.opponent_id),
            };
            for cmp in self.components.iter_mut() {
                cmp.process_command(&command, &mut self.data)?;
            }
        }

        Ok(())
    }

    fn render_gui(&mut self, ui: &mut Ui, painter: &Painter) -> Option<Scene> {
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let sidebar_w = realm_rect().map(|r| r.min.x).unwrap_or(220.0);
        let is_in_turn = self.current_player == self.data.player_id;
        let is_idle = matches!(self.data.status, Status::Idle);

        // Turn indicator
        let (turn_label, turn_color) = if is_in_turn {
            ("YOUR TURN", Color32::from_rgb(89, 242, 102))
        } else {
            ("THEIR TURN", Color32::from_rgb(153, 153, 166))
        };
        painter.text(
            pos2(sidebar_w / 2.0, 120.0),
            egui::Align2::CENTER_TOP,
            turn_label,
            FontId::proportional(18.0),
            turn_color,
        );

        // Action buttons — placed above the player status panel with a gap
        // Panel top is at: sr.height() - SIDEBAR_PANEL_RESERVED
        // Button height ~48 px + 12 px gap above the panel
        let btn_y = sr.height() - SIDEBAR_PANEL_RESERVED - 48.0 - 12.0;
        let btn_pos = pos2(10.0, btn_y);

        if is_in_turn && is_idle {
            let client = self.client.clone();
            let player_id = self.data.player_id;
            let game_id = self.game_id;
            egui::Area::new(egui::Id::new("pass_turn_btn"))
                .fixed_pos(btn_pos)
                .show(ui, |ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("Pass Turn")
                            .size(18.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(vec2(160.0, 48.0));
                    if ui.add(btn).clicked() {
                        client
                            .send(ClientMessage::EndTurn { player_id, game_id })
                            .ok();
                    }
                });
        } else if matches!(
            self.data.status,
            Status::SelectingCard { multiple: true, .. }
        ) || self.data.status == Status::Mulligan
        {
            let mut done = false;
            egui::Area::new(egui::Id::new("done_selecting_btn"))
                .fixed_pos(btn_pos)
                .show(ui, |ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("Done Selecting")
                            .size(18.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(vec2(180.0, 48.0));
                    if ui.add(btn).clicked() {
                        done = true;
                    }
                });
            if done {
                for component in &mut self.components {
                    let _ =
                        component.process_command(&ComponentCommand::DonePicking, &mut self.data);
                }
            }
        }

        // Overlays for waiting/selecting action/game aborted
        let needs_overlay = matches!(
            &self.data.status,
            Status::Waiting { .. } | Status::SelectingAction { .. } | Status::GameAborted { .. }
        );
        if needs_overlay {
            painter.rect_filled(
                Rect::from_min_size(pos2(0.0, 0.0), vec2(sr.width(), sr.height())),
                0.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 204),
            );
        }

        match &self.data.status.clone() {
            Status::Waiting { prompt } => {
                painter.text(
                    sr.center(),
                    egui::Align2::CENTER_CENTER,
                    prompt,
                    FontId::proportional(FONT_SIZE),
                    Color32::WHITE,
                );
                None
            }
            Status::SelectingAmount {
                prompt,
                min_amount,
                max_amount,
            } => {
                if self.selected_value.is_none() {
                    self.selected_value = Some(Box::new(*min_amount as i32));
                }

                let selected_amount = self
                    .selected_value
                    .as_mut()
                    .unwrap()
                    .downcast_mut::<i32>()
                    .unwrap();
                let mut submitted = false;
                let menu_w = 260.0;
                let menu_h = 170.0;
                let screen = screen_rect()
                    .unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0)));
                let origin = pos2(
                    (screen.width() - menu_w) / 2.0,
                    (screen.height() - menu_h) / 2.0,
                );
                egui::Area::new(egui::Id::new("amount_picker_popup"))
                    .fixed_pos(origin)
                    .order(egui::Order::Foreground)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new(prompt)
                                        .size(16.0)
                                        .color(Color32::from_rgb(180, 200, 240)),
                                );
                                ui.add_space(18.0);
                                if ui
                                    .add_enabled(
                                        *selected_amount > *min_amount as i32,
                                        egui::Button::new("-").min_size(vec2(32.0, 32.0)),
                                    )
                                    .clicked()
                                {
                                    *selected_amount -= 1;
                                }
                                let amt_field = egui::DragValue::new(selected_amount)
                                    .range(*min_amount as i32..=*max_amount as i32)
                                    .speed(1)
                                    .fixed_decimals(0)
                                    .min_decimals(0)
                                    .max_decimals(0)
                                    .prefix("")
                                    .suffix("");
                                ui.add_sized([60.0, 32.0], amt_field);
                                if ui
                                    .add_enabled(
                                        *selected_amount < *max_amount as i32,
                                        egui::Button::new("+").min_size(vec2(32.0, 32.0)),
                                    )
                                    .clicked()
                                {
                                    *selected_amount += 1;
                                }
                                ui.add_space(18.0);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Submit")
                                                .size(18.0)
                                                .color(Color32::WHITE),
                                        )
                                        .min_size(vec2(120.0, 36.0)),
                                    )
                                    .clicked()
                                {
                                    submitted = true;
                                }
                            });
                        });
                    });
                if submitted {
                    self.client
                        .send(ClientMessage::PickAmount {
                            game_id: self.game_id,
                            player_id: self.data.player_id,
                            amount: *selected_amount as u8,
                        })
                        .ok();
                    self.data.status = Status::Idle;
                }
                None
            }
            Status::SelectingAction {
                prompt,
                actions,
                anchor_on_cursor,
                ..
            } => {
                let pos = if *anchor_on_cursor {
                    self.data.last_clicked_card_pos
                } else {
                    None
                };
                let result = popup_action_menu(ui, pos, prompt, actions, painter);
                if let Some(idx) = result {
                    self.client
                        .send(ClientMessage::PickAction {
                            game_id: self.game_id,
                            player_id: self.data.player_id,
                            action_idx: idx,
                        })
                        .ok();
                    self.data.status = Status::Idle;
                }
                None
            }
            Status::GameAborted { reason } => {
                let reason = reason.clone();
                let mut new_scene = None;
                egui::Window::new("Game Aborted")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .show(ui, |ui| {
                        for line in reason.lines() {
                            ui.label(RichText::new(line).size(12.0));
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Ok").size(18.0).color(Color32::WHITE),
                                )
                                .min_size(vec2(80.0, 24.0)),
                            )
                            .clicked()
                        {
                            new_scene = Some(Scene::Menu(Menu::new(self.client.clone())));
                        }
                    });
                if new_scene.is_some() {
                    self.data.status = Status::Idle;
                }
                new_scene
            }
            _ => None,
        }
    }

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

                self.open_viewers(cards)
                    .expect("Failed to compute viewers for card selection");

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
                resources,
                health,
            } => {
                self.data.cards = sort_cards(cards);
                self.current_player = *current_player;
                self.data.resources = resources.clone();
                self.data.avatar_health = health.clone();
                None
            }
            ServerMessage::ForceSync {
                cards,
                current_player,
                resources,
                health,
                ..
            } => {
                self.data.cards = sort_cards(cards);
                self.current_player = *current_player;
                self.data.resources = resources.clone();
                self.data.avatar_health = health.clone();
                None
            }
            _ => None,
        }
    }
}
