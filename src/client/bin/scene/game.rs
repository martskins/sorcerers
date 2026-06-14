use super::selection_overlay::SelectionOverlayBehaviour;
use crate::{
    components::{
        Component, ComponentCommand, ComponentType, card_viewer::CardViewerComponent,
        event_log::EventLogComponent, player_hand::PlayerHandComponent,
        player_status::PlayerStatusComponent, realm::RealmComponent,
    },
    config::*,
    render::{ActionMenuResponse, popup_action_menu},
    scene::{
        Scene,
        action_overlay::ActionOverlay,
        card_toast::{CardToast, TOAST_MARGIN},
        combat_resolution_overlay::CombatResolutionOverlay,
        menu::Menu,
        selection_overlay::SelectionOverlay,
    },
    texture_cache::TextureCache,
    theme,
};
use egui::{Color32, Context, FontId, Painter, Rect, RichText, Ui, pos2, vec2};
use kira::{AudioManager, DefaultBackend, sound::static_sound::StaticSoundData};
use sorcerers::{
    card::{CardData, CardType, Region},
    game::{CardId, Direction, PlayerId, Resources},
    networking::{
        self,
        message::{ClientMessage, EffectDebugData, OngoingEffectData, ServerMessage},
    },
    zone::{Location, Zone},
};
use std::collections::HashMap;

mod messages;
mod ui;

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
        source_card_id: Option<CardId>,
        anchor_on_cursor: bool,
    },
    SelectingDirection {
        directions: Vec<Direction>,
        prompt: String,
        source_card_id: Option<CardId>,
    },
    SelectingCard {
        cards: Vec<CardId>,
        pickable_cards: Vec<CardId>,
        preview: bool,
        prompt: String,
        source_card_id: Option<CardId>,
        multiple: bool,
    },
    SelectingAmount {
        prompt: String,
        source_card_id: Option<CardId>,
        min_amount: u8,
        max_amount: u8,
    },
    SelectingPath {
        paths: Vec<Vec<Location>>,
        prompt: String,
        source_card_id: Option<CardId>,
    },
    SelectingZoneGroup {
        groups: Vec<Vec<Location>>,
        prompt: String,
        source_card_id: Option<CardId>,
    },
    SelectingZone {
        locations: Vec<Location>,
        prompt: String,
        source_card_id: Option<CardId>,
    },
    PreviewingPlayableLocations {
        card_id: CardId,
        locations: Vec<Location>,
    },
    ViewingCards {
        cards: Vec<CardId>,
        prompt: String,
        prev_status: Box<Status>,
        behaviour: SelectionOverlayBehaviour,
    },
    DistributingDamage {
        player_id: PlayerId,
        attacker: CardId,
        defenders: Vec<CardId>,
        damage: u16,
    },
    GameAborted {
        reason: String,
    },
    GameOver {
        winner_id: PlayerId,
        winner_name: String,
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

#[derive(Debug, Clone)]
pub struct PendingProjectileAnimation {
    pub id: uuid::Uuid,
    pub shooter: CardId,
    pub path: Vec<Location>,
    pub direction: Direction,
    pub ranged_strike: bool,
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
    pub aura_areas_of_effect: HashMap<uuid::Uuid, Option<Vec<Location>>>,
    pub ongoing_effects: Option<Vec<OngoingEffectData>>,
    pub show_ongoing_effects: bool,
    pub show_controls_help: bool,
    pub highlighted_ongoing_effect: Option<OngoingEffectData>,
    /// Screen position of the last card the player clicked; used to anchor context menus.
    pub last_clicked_card_pos: Option<egui::Pos2>,
    pub last_clicked_card_rect: Option<egui::Rect>,
    pub last_clicked_cursor_pos: Option<egui::Pos2>,
    pub last_clicked_card_id: Option<CardId>,
    pub last_clicked_card_time: Option<f64>,
    pub pending_projectiles: Vec<PendingProjectileAnimation>,
    pub stepped_effects: bool,
    pub effect_queue: Vec<EffectDebugData>,
    pub show_debug_effects: bool,
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
            aura_areas_of_effect: HashMap::new(),
            ongoing_effects: None,
            show_ongoing_effects: false,
            show_controls_help: false,
            highlighted_ongoing_effect: None,
            last_clicked_card_pos: None,
            last_clicked_card_rect: None,
            last_clicked_cursor_pos: None,
            last_clicked_card_id: None,
            last_clicked_card_time: None,
            pending_projectiles: Vec::new(),
            stepped_effects: false,
            effect_queue: Vec::new(),
            show_debug_effects: false,
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
    client: networking::client::Client,
    current_player: PlayerId,
    overlay: Option<GameOverlay>,
    components: GameComponents,
    data: GameData,
    audio_manager: AudioManager<DefaultBackend>,
    selected_value: Option<Box<dyn std::any::Any>>,
    card_toast: Vec<CardToast>,
    prompt_stack_pos: Option<egui::Pos2>,
    controlled_hand_opened_for: Option<PlayerId>,
}

enum GameOverlay {
    Action(ActionOverlay),
    Selection(SelectionOverlay),
    CombatResolution(CombatResolutionOverlay),
}

impl GameOverlay {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        match self {
            Self::Action(overlay) => overlay.update(data, ctx),
            Self::Selection(overlay) => overlay.update(data, ctx),
            Self::CombatResolution(overlay) => overlay.update(data, ctx),
        }
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        match self {
            Self::Action(overlay) => overlay.render(data, ui, painter),
            Self::Selection(overlay) => overlay.render(data, ui, painter),
            Self::CombatResolution(overlay) => overlay.render(data, ui, painter),
        }
    }
}

struct GameComponents {
    opponent_status: PlayerStatusComponent,
    player_status: PlayerStatusComponent,
    realm: RealmComponent,
    hand: PlayerHandComponent,
    event_log: EventLogComponent,
    card_viewer: CardViewerComponent,
}

impl GameComponents {
    fn new(
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        opponent_id: &PlayerId,
        is_player_one: bool,
        client: networking::client::Client,
    ) -> Self {
        let player_status_rect = component_rect(ComponentType::PlayerStatus).unwrap_or(Rect::ZERO);
        let realm_r = component_rect(ComponentType::Realm).unwrap_or(Rect::ZERO);
        let hand_r = component_rect(ComponentType::PlayerHand).unwrap_or(Rect::ZERO);
        let log_rect = event_log_rect();

        Self {
            opponent_status: PlayerStatusComponent::new(player_status_rect, *opponent_id, false),
            player_status: PlayerStatusComponent::new(player_status_rect, *player_id, true),
            realm: RealmComponent::new(game_id, player_id, !is_player_one, client.clone(), realm_r),
            hand: PlayerHandComponent::new(game_id, player_id, client.clone(), hand_r),
            event_log: EventLogComponent::new(log_rect),
            card_viewer: CardViewerComponent::new(game_id, player_id, client),
        }
    }

    fn update(&mut self, data: &mut GameData, ctx: &Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
            data.show_debug_effects = !data.show_debug_effects;
        }

        Self::update_component(&mut self.opponent_status, data, ctx);
        Self::update_component(&mut self.player_status, data, ctx);
        Self::update_component(&mut self.realm, data, ctx);
        Self::update_component(&mut self.hand, data, ctx);
        Self::update_component(&mut self.event_log, data, ctx);
        Self::update_component(&mut self.card_viewer, data, ctx);
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
    ) -> Vec<ComponentCommand> {
        let mut actions = Vec::new();
        Self::render_component(&mut self.opponent_status, data, ui, painter, &mut actions);
        Self::render_component(&mut self.player_status, data, ui, painter, &mut actions);
        Self::render_component(&mut self.realm, data, ui, painter, &mut actions);
        Self::render_component(&mut self.hand, data, ui, painter, &mut actions);
        Self::render_component(&mut self.event_log, data, ui, painter, &mut actions);
        Self::render_component(&mut self.card_viewer, data, ui, painter, &mut actions);
        actions
    }

    fn process_command(
        &mut self,
        command: &ComponentCommand,
        data: &mut GameData,
    ) -> anyhow::Result<()> {
        self.opponent_status.process_command(command, data)?;
        self.player_status.process_command(command, data)?;
        self.realm.process_command(command, data)?;
        self.hand.process_command(command, data)?;
        self.event_log.process_command(command, data)?;
        self.card_viewer.process_command(command, data)?;
        Ok(())
    }

    fn update_component<C: Component>(component: &mut C, data: &mut GameData, ctx: &Context) {
        if let Err(e) = component.update(data, ctx) {
            eprintln!("Error updating component: {}", e);
        }
        if let Ok(rect) = component_rect(component.get_component_type()) {
            let _ = component.process_command(
                &ComponentCommand::SetRect {
                    component_type: component.get_component_type(),
                    rect,
                },
                data,
            );
        }
    }

    fn render_component<C: Component>(
        component: &mut C,
        data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
        actions: &mut Vec<ComponentCommand>,
    ) {
        if !component.is_visible() {
            return;
        }

        match component.render(data, ui, painter) {
            Ok(Some(action)) => actions.push(action),
            Ok(None) => {}
            Err(e) => eprintln!("Error rendering component: {}", e),
        }
    }
}

impl Game {
    pub fn new(
        game_id: uuid::Uuid,
        player_id: PlayerId,
        opponent_id: PlayerId,
        is_player_one: bool,
        cards: Vec<CardData>,
        client: networking::client::Client,
        audio_manager: AudioManager<DefaultBackend>,
    ) -> Self {
        Self {
            game_id,
            player_id,
            client: client.clone(),
            current_player: uuid::Uuid::nil(),
            overlay: None,
            components: GameComponents::new(
                &game_id,
                &player_id,
                &opponent_id,
                is_player_one,
                client.clone(),
            ),
            data: GameData::new(&player_id, cards),
            audio_manager,
            selected_value: None,
            card_toast: Vec::new(),
            prompt_stack_pos: None,
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
        self.components.update(&mut self.data, ctx);

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
            self.overlay = Some(GameOverlay::Selection(SelectionOverlay::new(
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

        let component_actions = self.components.render(&mut self.data, ui, &painter);

        for action in component_actions {
            let _ = self.components.process_command(&action, &mut self.data);
        }
        let new_scene = self.render_gui(ui, &painter);
        self.render_prompt_stack(ui);

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
        let mut viewers: Vec<(Zone, PlayerId, Vec<CardId>)> = Vec::new();
        for card in self.data.cards.iter().filter(|card| cards.contains(&card.id)) {
            if card.zone.is_in_play() {
                continue;
            }
            if let Some((_zone, _owner_id, card_ids)) = viewers
                .iter_mut()
                .find(|(zone, owner_id, _)| zone == &card.zone && owner_id == &card.owner_id)
            {
                card_ids.push(card.id);
            } else {
                viewers.push((card.zone.clone(), card.owner_id, vec![card.id]));
            }
        }

        for (zone, owner_id, card_ids) in viewers {
            let owner_label = if owner_id == self.player_id {
                "Your"
            } else {
                "Opponent's"
            };
            let zone_label = match zone {
                Zone::Hand => "Hand",
                Zone::Cemetery => "Cemetery",
                Zone::Spellbook => "Spellbook",
                Zone::Atlasbook => "Atlas",
                Zone::Banish => "Banished Cards",
                _ => "Cards",
            };
            let command = ComponentCommand::OpenCardViewer {
                title: format!("{} {}", owner_label, zone_label),
                zone,
                controller_id: Some(owner_id),
                card_ids: Some(card_ids),
                open_only: true,
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
            card_ids: None,
            open_only: true,
        };
        self.broadcast_command(&command);
        self.controlled_hand_opened_for = Some(self.data.turn_player);
    }

    fn broadcast_command(&mut self, command: &ComponentCommand) {
        if let Err(e) = self.components.process_command(command, &mut self.data) {
            eprintln!("Error processing component command: {}", e);
        }
    }

    fn current_prompt(&self) -> Option<(&str, Option<CardId>)> {
        match &self.data.status {
            Status::Waiting { prompt } => Some((prompt.as_str(), None)),
            Status::SelectingZone {
                prompt,
                source_card_id,
                ..
            }
            | Status::SelectingZoneGroup {
                prompt,
                source_card_id,
                ..
            }
            | Status::SelectingPath {
                prompt,
                source_card_id,
                ..
            }
            | Status::SelectingAmount {
                prompt,
                source_card_id,
                ..
            }
            | Status::SelectingDirection {
                prompt,
                source_card_id,
                ..
            } => Some((prompt.as_str(), *source_card_id)),
            Status::SelectingCard {
                prompt,
                source_card_id,
                ..
            } => Some((prompt.as_str(), *source_card_id)),
            _ => None,
        }
    }

    fn render_prompt_stack(&mut self, ui: &mut Ui) {
        let Some((prompt, source_card_id)) = self.current_prompt() else {
            return;
        };

        let ctx = ui.ctx().clone();
        let card =
            source_card_id.and_then(|id| self.data.cards.iter().find(|c| c.id == id).cloned());
        let instruction = prompt.to_string();
        const PAD: f32 = 10.0;
        const CARD_W: f32 = 86.0;
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let has_card = card.is_some();
        let panel_w = (if has_card { 380.0_f32 } else { 300.0_f32 }).min(sr.width() - 32.0);
        let panel_h = if has_card { 138.0 } else { 92.0 };
        let default_pos = pos2(sr.min.x + 18.0, sr.min.y + 72.0);
        let pos = self.prompt_stack_pos.get_or_insert(default_pos);
        pos.x = pos.x.clamp(sr.min.x + 8.0, sr.max.x - panel_w - 8.0);
        pos.y = pos.y.clamp(sr.min.y + 8.0, sr.max.y - panel_h - 8.0);
        let rect = Rect::from_min_size(*pos, vec2(panel_w, panel_h));
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("prompt_stack_window"),
        ));
        let response = ui.interact(
            rect,
            egui::Id::new("prompt_stack_drag_handle"),
            egui::Sense::click_and_drag(),
        );
        if response.dragged() {
            *pos += response.drag_delta();
            pos.x = pos.x.clamp(sr.min.x + 8.0, sr.max.x - panel_w - 8.0);
            pos.y = pos.y.clamp(sr.min.y + 8.0, sr.max.y - panel_h - 8.0);
        }
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }

        painter.rect_filled(rect, 7.0, Color32::from_rgba_unmultiplied(7, 9, 18, 230));
        painter.rect_stroke(
            rect,
            7.0,
            egui::Stroke::new(1.0, theme::PANEL_BORDER),
            egui::StrokeKind::Outside,
        );

        let image_rect = Rect::from_min_size(
            rect.min + vec2(PAD, PAD),
            vec2(CARD_W, CARD_W / CARD_ASPECT_RATIO),
        );
        if let Some(card) = &card {
            if let Some(tex) = TextureCache::get_card_texture_blocking(card, &ctx) {
                let mut draw_rect = image_rect;
                if tex.aspect_ratio() > 1.0 {
                    draw_rect = Rect::from_min_size(
                        image_rect.min,
                        vec2(CARD_W, CARD_W * CARD_ASPECT_RATIO),
                    );
                }
                painter.image(
                    tex.id(),
                    draw_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                painter.rect_filled(image_rect, 4.0, Color32::from_rgb(42, 48, 68));
            }
        }

        let text_x = if has_card {
            image_rect.max.x + 14.0
        } else {
            rect.min.x + PAD
        };
        let text_w = rect.max.x - text_x - PAD;
        painter.text(
            pos2(text_x, rect.min.y + PAD),
            egui::Align2::LEFT_TOP,
            card.as_ref()
                .map(|card| card.name.as_str())
                .unwrap_or("Pending choice"),
            FontId::proportional(16.0),
            theme::TEXT_BRIGHT,
        );
        if has_card {
            painter.text(
                pos2(text_x, rect.min.y + PAD + 22.0),
                egui::Align2::LEFT_TOP,
                "Triggered ability",
                FontId::proportional(11.0),
                Color32::from_rgb(132, 168, 215),
            );
        }
        let galley = ctx.fonts_mut(|f| {
            f.layout(
                instruction,
                FontId::proportional(13.0),
                Color32::from_rgb(214, 224, 245),
                text_w,
            )
        });
        let text_y = if has_card {
            rect.min.y + PAD + 44.0
        } else {
            rect.min.y + PAD + 28.0
        };
        painter.galley(
            pos2(text_x, text_y),
            galley,
            Color32::from_rgb(214, 224, 245),
        );
    }

    fn broadcast_command_result(&mut self, command: &ComponentCommand) -> anyhow::Result<()> {
        self.components.process_command(command, &mut self.data)
    }
}
