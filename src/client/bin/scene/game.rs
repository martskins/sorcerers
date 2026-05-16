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
    theme,
};
use egui::{Color32, Context, FontId, Painter, Rect, RichText, Stroke, Ui, pos2, vec2};
use kira::{AudioManager, DefaultBackend, sound::static_sound::StaticSoundData};
use sorcerers::{
    card::{CardData, CardType, Region},
    game::{PlayerId, Resources},
    networking::{
        self,
        message::{ClientMessage, ServerMessage},
    },
    zone::Zone,
};
use std::collections::HashMap;

mod messages;
mod ui;

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
    PreviewingPlayableZones {
        card_id: uuid::Uuid,
        zones: Vec<Zone>,
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
    pub current_player: PlayerId,
    pub turn_player: PlayerId,
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
            current_player: uuid::Uuid::nil(),
            turn_player: uuid::Uuid::nil(),
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
    controlled_hand_opened_for: Option<PlayerId>,
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
            controlled_hand_opened_for: None,
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
            && let Err(e) = overlay.update(&mut self.data, ctx)
        {
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
            if !component.is_visible() {
                continue;
            }

            match component.render(&mut self.data, ui, &painter) {
                Ok(Some(action)) => component_actions.push(action),
                Ok(None) => {}
                Err(e) => eprintln!("Error rendering component: {}", e),
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

        if let Some(overlay) = &mut self.overlay {
            match overlay.render(&mut self.data, ui, &painter) {
                Ok(Some(ComponentCommand::CloseOverlay)) => {
                    self.overlay = None;
                }
                Ok(_) => {}
                Err(e) => eprintln!("Error rendering overlay: {}", e),
            }
        }

        new_scene
    }

    fn open_viewers(&mut self, cards: &[uuid::Uuid]) -> anyhow::Result<()> {
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
                open_only: false,
            };
            self.broadcast_command_result(&command)?;
        }

        if open_opponent_cemetery {
            let command = ComponentCommand::OpenCardViewer {
                title: "Opponent's Cemetery".to_string(),
                zone: Zone::Cemetery,
                controller_id: Some(self.opponent_id),
                open_only: false,
            };
            self.broadcast_command_result(&command)?;
        }

        Ok(())
    }

    fn open_controlled_hand_viewer(&mut self) {
        if self.current_player != self.player_id || self.data.turn_player == self.player_id {
            self.controlled_hand_opened_for = None;
            return;
        }
        if self.controlled_hand_opened_for == Some(self.data.turn_player) {
            return;
        }

        let command = ComponentCommand::OpenCardViewer {
            title: "Controlled Player's Hand".to_string(),
            zone: Zone::Hand,
            controller_id: Some(self.data.turn_player),
            open_only: true,
        };
        self.broadcast_command(&command);
        self.controlled_hand_opened_for = Some(self.data.turn_player);
    }

    fn broadcast_command(&mut self, command: &ComponentCommand) {
        for component in &mut self.components {
            if let Err(e) = component.process_command(command, &mut self.data) {
                eprintln!("Error processing component command: {}", e);
            }
        }
    }

    fn broadcast_command_result(&mut self, command: &ComponentCommand) -> anyhow::Result<()> {
        for component in &mut self.components {
            component.process_command(command, &mut self.data)?;
        }
        Ok(())
    }
}
