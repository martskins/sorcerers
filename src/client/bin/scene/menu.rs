use crate::scene::{Scene, game::Game};
use crate::texture_cache::TextureCache;
use crate::theme;
use egui::{Color32, Context, TextureHandle, TextureOptions, Ui, pos2, vec2};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use sorcerers::booster::{BoosterCard, BoosterPack, UnopenedBoosterPack};
use sorcerers::card::{CardData, Region, from_name};
use sorcerers::collection::CollectedCard;
use sorcerers::deck::DeckList;
use sorcerers::deck::precon::PreconDeck;
use sorcerers::game::PlayerId;
use sorcerers::networking::message::ServerMessage;
use sorcerers::networking::{
    self,
    message::{ClientMessage, DeckChoice},
};

const MENU_BG: Color32 = Color32::from_rgb(8, 8, 14);
const MENU_BORDER: Color32 = theme::PANEL_BORDER;
const MENU_TEXT: Color32 = Color32::from_rgb(235, 236, 225);
const MENU_TEXT_MUTED: Color32 = Color32::from_rgb(171, 179, 168);
const MENU_GOLD: Color32 = Color32::from_rgb(255, 200, 60);
const BETA_BOOSTER_COST: u32 = 30;
const MENU_BACKGROUND: &[u8] =
    include_bytes!("../../../../assets/images/menu/enchanted_table_v1.png");

pub struct Menu {
    client: networking::client::Client,
    player_id: Option<PlayerId>,
    available_decks: Vec<PreconDeck>,
    saved_decks: Vec<DeckList>,
    collection: Vec<CollectedCard>,
    selected_saved_deck: Option<usize>,
    deck_error: Option<String>,
    looking_for_match: bool,
    player_name: String,
    username: String,
    email: String,
    password: String,
    confirmation_code: String,
    registering: bool,
    awaiting_email_confirmation: bool,
    auth_requested: bool,
    auth_error: Option<String>,
    booster_reward: Option<String>,
    reward_points: u32,
    show_rewards: bool,
    reward_redemption_requested: bool,
    reward_feedback: Option<String>,
    unopened_booster_packs: Vec<UnopenedBoosterPack>,
    opened_booster_pack: Option<BoosterPack>,
    show_packs: bool,
    selecting_starter_deck: bool,
    starter_decks: Vec<PreconDeck>,
    connect_requested: bool,
    menu_background: Option<TextureHandle>,
    #[cfg(feature = "name-entry")]
    /// Time (seconds, from `ctx.input`) when the shake was triggered.
    shake_start: Option<f64>,
    /// True while the name field is empty after a failed submit attempt.
    show_name_error: bool,
}

impl std::fmt::Debug for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Menu")
            .field("player_id", &self.player_id)
            .field("available_decks", &self.available_decks)
            .field("saved_decks", &self.saved_decks)
            .field("looking_for_match", &self.looking_for_match)
            .field("player_name", &self.player_name)
            .field("registering", &self.registering)
            .field("auth_requested", &self.auth_requested)
            .finish_non_exhaustive()
    }
}

impl Menu {
    pub(crate) fn set_reward_points(&mut self, reward_points: u32) {
        self.reward_points = reward_points;
    }

    pub fn new(client: networking::client::Client) -> Self {
        Self {
            client,
            player_id: None,
            available_decks: vec![],
            saved_decks: DeckList::load_all(),
            collection: vec![],
            selected_saved_deck: None,
            deck_error: None,
            looking_for_match: false,
            player_name: String::new(),
            username: String::new(),
            email: String::new(),
            password: String::new(),
            confirmation_code: String::new(),
            registering: false,
            awaiting_email_confirmation: false,
            auth_requested: false,
            auth_error: None,
            booster_reward: None,
            reward_points: 0,
            show_rewards: false,
            reward_redemption_requested: false,
            reward_feedback: None,
            unopened_booster_packs: vec![],
            opened_booster_pack: None,
            show_packs: false,
            selecting_starter_deck: false,
            starter_decks: vec![],
            connect_requested: false,
            menu_background: None,
            #[cfg(feature = "name-entry")]
            shake_start: None,
            show_name_error: false,
        }
    }

    /// Restore menu state without adding a custom deck (used by Back button).
    pub fn restore(
        client: networking::client::Client,
        player_id: Option<PlayerId>,
        player_name: String,
        available_decks: Vec<PreconDeck>,
        saved_decks: Vec<DeckList>,
        collection: Vec<CollectedCard>,
    ) -> Self {
        Self {
            client,
            player_id,
            available_decks,
            saved_decks,
            collection,
            selected_saved_deck: None,
            deck_error: None,
            looking_for_match: false,
            player_name,
            username: String::new(),
            email: String::new(),
            password: String::new(),
            confirmation_code: String::new(),
            registering: false,
            awaiting_email_confirmation: false,
            auth_requested: false,
            auth_error: None,
            booster_reward: None,
            reward_points: 0,
            show_rewards: false,
            reward_redemption_requested: false,
            reward_feedback: None,
            unopened_booster_packs: vec![],
            opened_booster_pack: None,
            show_packs: false,
            selecting_starter_deck: false,
            starter_decks: vec![],
            connect_requested: false,
            menu_background: None,
            #[cfg(feature = "name-entry")]
            shake_start: None,
            show_name_error: false,
        }
    }

    pub fn update(&mut self, _ctx: &Context) {}

    fn render_menu_background(&mut self, ui: &mut Ui) {
        if self.menu_background.is_none() {
            let image = image::load_from_memory(MENU_BACKGROUND)
                .expect("menu background asset should decode")
                .to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
            self.menu_background = Some(ui.ctx().load_texture(
                "menu_enchanted_table",
                color_image,
                TextureOptions::LINEAR,
            ));
        }

        let rect = ui.max_rect();
        let texture = self
            .menu_background
            .as_ref()
            .expect("menu background should be loaded");
        let image_size = texture.size_vec2();
        let viewport_ratio = rect.width() / rect.height().max(1.0);
        let image_ratio = image_size.x / image_size.y.max(1.0);
        let uv = if viewport_ratio > image_ratio {
            let visible_height = image_ratio / viewport_ratio;
            let inset = (1.0 - visible_height) * 0.5;
            egui::Rect::from_min_max(pos2(0.0, inset), pos2(1.0, 1.0 - inset))
        } else {
            let visible_width = viewport_ratio / image_ratio;
            let inset = (1.0 - visible_width) * 0.5;
            egui::Rect::from_min_max(pos2(inset, 0.0), pos2(1.0 - inset, 1.0))
        };
        ui.painter().image(
            texture.id(),
            rect,
            uv,
            Color32::from_rgba_unmultiplied(255, 255, 255, 210),
        );
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(3, 6, 10, 72));
    }

    fn render_brand_heading(&self, ui: &mut Ui, compact: bool) {
        ui.label(
            egui::RichText::new("✦  Sorcerers  ✦")
                .color(MENU_GOLD)
                .font(theme::display_bold_font(if compact { 44.0 } else { 58.0 })),
        );
        ui.add_space(if compact { 6.0 } else { 10.0 });
        ui.label(
            egui::RichText::new("Play Sorcery online. The rules are handled.")
                .color(MENU_TEXT_MUTED)
                .font(theme::display_font(if compact { 16.0 } else { 18.0 })),
        );
    }

    fn render_auth_input(ui: &mut Ui, value: &mut String, hint: &str, password: bool) {
        let frame = egui::Frame::new()
            .fill(theme::SURFACE_INSET)
            .stroke(egui::Stroke::NONE)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(12, 9));
        let response = ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .password(password)
                .desired_width(336.0)
                .frame(frame)
                .background_color(Color32::TRANSPARENT),
        );
        let (stroke, width) = if response.has_focus() {
            (Color32::from_rgb(116, 190, 229), 1.5)
        } else if response.hovered() {
            (Color32::from_rgb(105, 119, 113), 1.0)
        } else {
            (theme::INPUT_BORDER, 1.0)
        };
        ui.painter().rect_stroke(
            response.rect,
            6.0,
            egui::Stroke::new(width, stroke),
            egui::StrokeKind::Inside,
        );
    }

    fn render_auth_card(&mut self, ui: &mut Ui) {
        if self.awaiting_email_confirmation {
            self.render_email_confirmation_card(ui);
            return;
        }
        let title = if self.registering {
            "Create your player account"
        } else {
            "Welcome back"
        };
        let supporting_copy = if self.registering {
            "Create an account to start building a collection and playing online."
        } else {
            "Log in to choose a deck and join a match."
        };

        egui::Frame::new()
            .fill(Color32::from_rgba_premultiplied(7, 11, 11, 176))
            .stroke(egui::Stroke::NONE)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(24))
            .show(ui, |ui| {
                ui.set_width(360.0);
                ui.label(
                    egui::RichText::new(title)
                        .color(MENU_TEXT)
                        .size(24.0)
                        .strong(),
                );
                ui.add_space(7.0);
                ui.label(
                    egui::RichText::new(supporting_copy)
                        .color(MENU_TEXT_MUTED)
                        .size(14.0),
                );
                ui.add_space(24.0);

                ui.label(
                    egui::RichText::new("Username")
                        .color(MENU_TEXT)
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(6.0);
                Self::render_auth_input(ui, &mut self.username, "Choose a username", false);
                ui.add_space(16.0);
                if self.registering {
                    ui.label(
                        egui::RichText::new("Email address")
                            .color(MENU_TEXT)
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    Self::render_auth_input(
                        ui,
                        &mut self.email,
                        "you@example.com",
                        false,
                    );
                    ui.add_space(16.0);
                }
                ui.label(
                    egui::RichText::new("Password")
                        .color(MENU_TEXT)
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(6.0);
                Self::render_auth_input(ui, &mut self.password, "Enter your password", true);

                if let Some(error) = &self.auth_error {
                    ui.add_space(12.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(62, 29, 31))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(135, 55, 59)))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(error)
                                    .color(Color32::from_rgb(255, 195, 192))
                                    .size(14.0),
                            );
                        });
                }

                ui.add_space(22.0);
                let can_submit = !self.auth_requested
                    && !self.username.trim().is_empty()
                    && (!self.registering || !self.email.trim().is_empty())
                    && !self.password.is_empty();
                let submit_label = if self.auth_requested {
                    "Connecting…"
                } else if self.registering {
                    "Create account"
                } else {
                    "Log in"
                };
                let submit = ui.add_enabled(
                    can_submit,
                    egui::Button::new(egui::RichText::new(submit_label).size(17.0))
                        .min_size(vec2(360.0, theme::BUTTON_HEIGHT)),
                );
                if submit.clicked() {
                    let message = if self.registering {
                        ClientMessage::Register {
                            username: self.username.clone(),
                            email: self.email.trim().to_string(),
                            password: self.password.clone(),
                        }
                    } else {
                        ClientMessage::Login {
                            username: self.username.clone(),
                            password: self.password.clone(),
                        }
                    };
                    if self.client.send(message).is_ok() {
                        self.auth_requested = true;
                        self.auth_error = None;
                    } else {
                        self.auth_error = Some(
                            "Unable to reach the server. Check your connection and try again."
                                .to_string(),
                        );
                    }
                }

                ui.add_space(14.0);
                let switch_label = if self.registering {
                    "Already have an account? Log in"
                } else {
                    "New to Sorcerers? Create an account"
                };
                if ui
                    .link(egui::RichText::new(switch_label).color(Color32::from_rgb(122, 194, 245)))
                    .clicked()
                {
                    self.registering = !self.registering;
                    self.awaiting_email_confirmation = false;
                    self.auth_error = None;
                }
            });
    }

    fn render_email_confirmation_card(&mut self, ui: &mut Ui) {
        egui::Frame::new()
            .fill(Color32::from_rgba_premultiplied(7, 11, 11, 176))
            .stroke(egui::Stroke::NONE)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(24))
            .show(ui, |ui| {
                ui.set_width(360.0);
                ui.label(
                    egui::RichText::new("Confirm your email")
                        .color(MENU_TEXT)
                        .size(24.0)
                        .strong(),
                );
                ui.add_space(7.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Enter the six-digit code sent to {}. It expires after 15 minutes.",
                        self.email
                    ))
                    .color(MENU_TEXT_MUTED)
                    .size(14.0),
                );
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Confirmation code")
                        .color(MENU_TEXT)
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(6.0);
                Self::render_auth_input(
                    ui,
                    &mut self.confirmation_code,
                    "000000",
                    false,
                );
                if let Some(error) = &self.auth_error {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(error)
                            .color(Color32::from_rgb(255, 195, 192))
                            .size(14.0),
                    );
                }
                ui.add_space(22.0);
                let verify = ui.add_enabled(
                    !self.auth_requested && self.confirmation_code.len() == 6,
                    egui::Button::new(egui::RichText::new(if self.auth_requested {
                        "Confirming…"
                    } else {
                        "Confirm email"
                    })
                    .size(17.0))
                    .min_size(vec2(360.0, theme::BUTTON_HEIGHT)),
                );
                if verify.clicked() {
                    match self.client.send(ClientMessage::ConfirmEmail {
                        email: self.email.clone(),
                        code: self.confirmation_code.trim().to_string(),
                    }) {
                        Ok(()) => {
                            self.auth_requested = true;
                            self.auth_error = None;
                        }
                        Err(_) => {
                            self.auth_error = Some(
                                "Unable to reach the server. Check your connection and try again."
                                    .to_string(),
                            );
                        }
                    }
                }
                ui.add_space(12.0);
                let resend = ui
                    .add_enabled(
                        !self.auth_requested,
                        egui::Link::new(
                            egui::RichText::new("Resend confirmation code")
                                .color(Color32::from_rgb(122, 194, 245)),
                        ),
                    )
                    .clicked();
                if resend {
                    match self.client.send(ClientMessage::ResendEmailConfirmation {
                        email: self.email.clone(),
                    }) {
                        Ok(()) => {
                            self.auth_requested = true;
                            self.auth_error = None;
                        }
                        Err(_) => {
                            self.auth_error = Some(
                                "Unable to reach the server. Check your connection and try again."
                                    .to_string(),
                            );
                        }
                    }
                }
                ui.add_space(12.0);
                if ui
                    .link(
                        egui::RichText::new("Use a different email")
                            .color(Color32::from_rgb(122, 194, 245)),
                    )
                    .clicked()
                {
                    self.awaiting_email_confirmation = false;
                    self.registering = true;
                    self.confirmation_code.clear();
                    self.auth_error = None;
                }
            });
    }

    fn play_precon(&mut self, deck: PreconDeck) {
        self.deck_error = None;
        self.client
            .send(ClientMessage::JoinQueue {
                player_name: self.player_name.clone(),
                player_id: self.player_id.expect("player id should be set"),
                deck: DeckChoice::Precon(deck),
            })
            .ok();
        self.looking_for_match = true;
    }

    fn play_custom_deck(&mut self, deck_list: DeckList) {
        if let Some(starter_deck) = self
            .available_decks
            .iter()
            .find(|deck| deck_list.name == format!("{} Precon", deck.name()))
            .cloned()
        {
            self.play_precon(starter_deck);
            return;
        }

        match deck_list.validate() {
            Ok(()) => {
                self.deck_error = None;
                self.client
                    .send(ClientMessage::JoinQueue {
                        player_name: self.player_name.clone(),
                        player_id: self.player_id.expect("player id should be set"),
                        deck: DeckChoice::Custom(deck_list),
                    })
                    .ok();
                self.looking_for_match = true;
            }
            Err(msg) => {
                self.deck_error = Some(msg);
            }
        }
    }

    fn foil_cards_in_deck(&self, deck: &DeckList) -> u32 {
        deck.sites
            .iter()
            .chain(&deck.spells)
            .filter(|deck_card| deck_card.is_foil)
            .map(|deck_card| u32::from(deck_card.count))
            .sum()
    }

    fn render_deck_selection(&mut self, ui: &mut Ui, next_scene: &mut Option<Scene>) {
        ui.label(
            egui::RichText::new("Your decks")
                .color(Color32::from_rgb(220, 222, 245))
                .size(28.0),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Choose a deck to enter the queue.")
                .color(Color32::from_rgb(130, 145, 180))
                .size(15.0),
        );
        ui.add_space(18.0);

        let content_w = ui.available_width().min(860.0);
        let left_pad = ((ui.available_width() - content_w) / 2.0).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            ui.vertical(|ui| {
                ui.set_width(content_w);
                self.render_custom_section(ui, next_scene, content_w);

                if let Some(ref err) = self.deck_error.clone() {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("⚠ {err}"))
                            .color(Color32::from_rgb(220, 80, 60))
                            .size(14.0),
                    );
                }
            });
        });
    }

    fn render_reward_balance(&mut self, ui: &mut Ui) {
        let screen = ui.max_rect();
        egui::Area::new(egui::Id::new("reward_points_balance"))
            .fixed_pos(pos2(screen.right() - 188.0, screen.top() + 16.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                let response = ui.add(
                    egui::Button::new(
                        egui::RichText::new(format!("✦  {} points", self.reward_points))
                            .color(MENU_GOLD)
                            .size(15.0)
                            .strong(),
                    )
                    .min_size(vec2(172.0, 38.0)),
                );
                if response.clicked() {
                    self.show_rewards = true;
                    self.reward_feedback = None;
                }
                response.on_hover_text("View and claim match rewards");
            });
    }

    fn render_rewards_screen(&mut self, ui: &mut Ui) {
        let content_width = ui.available_width().min(1_040.0);
        let left_padding = ((ui.available_width() - content_width) / 2.0).max(0.0);

        ui.add_space(52.0);
        ui.horizontal(|ui| {
            ui.add_space(left_padding);
            ui.vertical(|ui| {
                ui.set_width(content_width);

                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("← Back").size(15.0))
                                .min_size(vec2(88.0, 40.0)),
                        )
                        .clicked()
                    {
                        self.show_rewards = false;
                        self.reward_feedback = None;
                    }
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Match rewards")
                                .color(MENU_TEXT)
                                .font(theme::display_bold_font(38.0)),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Build toward new cards every time you take a seat at the table.",
                            )
                            .color(MENU_TEXT_MUTED)
                            .size(15.0),
                        );
                    });
                });
                ui.add_space(26.0);

                egui::Frame::new()
                    .fill(theme::PANEL_BG)
                    .stroke(egui::Stroke::new(1.0, MENU_BORDER))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(24))
                    .show(ui, |ui| {
                        let compact = ui.available_width() < 760.0;
                        let mut render_booster = |ui: &mut Ui| {
                            ui.horizontal_top(|ui| {
                                let pack_size = vec2(152.0, 198.0);
                                let (pack_rect, _) =
                                    ui.allocate_exact_size(pack_size, egui::Sense::hover());
                                if let Some(texture) = TextureCache::get_texture_blocking(
                                    "assets/images/beta_booster_1.webp",
                                    ui.ctx(),
                                ) {
                                    egui::Image::new(egui::ImageSource::Texture(
                                        egui::load::SizedTexture::from_handle(&texture),
                                    ))
                                    .paint_at(ui, pack_rect);
                                } else {
                                    ui.painter()
                                        .rect_filled(pack_rect, 6.0, theme::SURFACE_INSET);
                                    ui.painter().rect_stroke(
                                        pack_rect,
                                        6.0,
                                        egui::Stroke::new(1.0, MENU_BORDER),
                                        egui::StrokeKind::Inside,
                                    );
                                    ui.painter().text(
                                        pack_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "BETA\nBOOSTER",
                                        theme::display_bold_font(18.0),
                                        MENU_GOLD,
                                    );
                                    ui.ctx().request_repaint();
                                }

                                ui.add_space(22.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("Beta Booster")
                                            .color(MENU_TEXT)
                                            .size(23.0)
                                            .strong(),
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "A fresh Beta pack, ready to join your collection.",
                                        )
                                        .color(MENU_TEXT_MUTED)
                                        .size(14.0),
                                    );
                                    ui.add_space(16.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{BETA_BOOSTER_COST} points"
                                        ))
                                        .color(MENU_GOLD)
                                        .size(17.0)
                                        .strong(),
                                    );
                                    ui.add_space(8.0);

                                    let progress = (self.reward_points as f32
                                        / BETA_BOOSTER_COST as f32)
                                        .min(1.0);
                                    let progress_width = ui.available_width().min(360.0);
                                    let (progress_rect, _) = ui.allocate_exact_size(
                                        vec2(progress_width, 30.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        progress_rect,
                                        5.0,
                                        theme::SURFACE_INSET,
                                    );
                                    if progress > 0.0 {
                                        let filled = egui::Rect::from_min_size(
                                            progress_rect.min,
                                            vec2(progress_rect.width() * progress, 30.0),
                                        );
                                        ui.painter().rect_filled(filled, 5.0, MENU_GOLD);
                                    }
                                    ui.painter().rect_stroke(
                                        progress_rect,
                                        5.0,
                                        egui::Stroke::new(1.0, MENU_BORDER),
                                        egui::StrokeKind::Inside,
                                    );
                                    ui.painter().text(
                                        progress_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        format!(
                                            "{} / {BETA_BOOSTER_COST} points",
                                            self.reward_points
                                        ),
                                        egui::FontId::proportional(14.0),
                                        MENU_TEXT,
                                    );
                                    ui.add_space(10.0);

                                    if self.reward_points < BETA_BOOSTER_COST {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} more points to claim this pack",
                                                BETA_BOOSTER_COST - self.reward_points
                                            ))
                                            .color(MENU_TEXT_MUTED)
                                            .size(14.0),
                                        );
                                    } else {
                                        ui.label(
                                            egui::RichText::new("Ready to claim")
                                                .color(theme::PICKABLE)
                                                .size(14.0)
                                                .strong(),
                                        );
                                    }
                                    ui.add_space(12.0);

                                    let claim = ui.add_enabled(
                                        self.reward_points >= BETA_BOOSTER_COST
                                            && !self.reward_redemption_requested,
                                        egui::Button::new(
                                            egui::RichText::new(
                                                if self.reward_redemption_requested {
                                                    "Claiming…"
                                                } else {
                                                    "Claim Beta Booster"
                                                },
                                            )
                                            .color(Color32::WHITE)
                                            .size(16.0),
                                        )
                                        .min_size(vec2(250.0, theme::BUTTON_HEIGHT)),
                                    );
                                    if claim.clicked() {
                                        match self.client.send(ClientMessage::RedeemBetaBooster) {
                                            Ok(()) => {
                                                self.reward_redemption_requested = true;
                                                self.reward_feedback = None;
                                            }
                                            Err(_) => {
                                                self.reward_feedback = Some(
                                                    "Unable to reach the server. Please try again."
                                                        .to_string(),
                                                );
                                            }
                                        }
                                    }
                                    if let Some(feedback) = &self.reward_feedback {
                                        ui.add_space(8.0);
                                        ui.label(
                                            egui::RichText::new(feedback)
                                                .color(MENU_GOLD)
                                                .size(13.0),
                                        );
                                    }
                                });
                            });
                        };

                        let render_summary = |ui: &mut Ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Your balance")
                                        .color(MENU_TEXT_MUTED)
                                        .size(13.0),
                                );
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new(format!("✦ {}", self.reward_points))
                                        .color(MENU_GOLD)
                                        .size(34.0)
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new("reward points")
                                        .color(MENU_TEXT_MUTED)
                                        .size(14.0),
                                );
                                ui.add_space(24.0);
                                ui.separator();
                                ui.add_space(18.0);
                                ui.label(
                                    egui::RichText::new("Earn points by playing")
                                        .color(MENU_TEXT)
                                        .size(15.0)
                                        .strong(),
                                );
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new("Win a match  +10")
                                        .color(MENU_TEXT_MUTED)
                                        .size(14.0),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new("Complete a match  +2")
                                        .color(MENU_TEXT_MUTED)
                                        .size(14.0),
                                );
                            });
                        };

                        if compact {
                            render_booster(ui);
                            ui.add_space(24.0);
                            ui.separator();
                            ui.add_space(20.0);
                            render_summary(ui);
                        } else {
                            ui.horizontal_top(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - 270.0).max(400.0));
                                    render_booster(ui);
                                });
                                ui.add_space(24.0);
                                ui.separator();
                                ui.add_space(24.0);
                                ui.vertical(|ui| {
                                    ui.set_width(220.0);
                                    render_summary(ui);
                                });
                            });
                        }
                    });
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(
                        "More sets and card rewards will appear here as they become available.",
                    )
                    .color(MENU_TEXT_MUTED)
                    .size(13.0),
                );
            });
        });
    }

    fn render_custom_section(
        &mut self,
        ui: &mut Ui,
        next_scene: &mut Option<Scene>,
        width: f32,
    ) {
        egui::Frame::new()
            .fill(theme::PANEL_BG)
            .stroke(egui::Stroke::new(1.0, MENU_BORDER))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(width - 28.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} deck{} ready",
                            self.saved_decks.len(),
                            if self.saved_decks.len() == 1 { "" } else { "s" }
                        ))
                            .color(Color32::from_rgb(125, 145, 180))
                            .size(14.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let build = ui.add(
                            egui::Label::new(
                                egui::RichText::new("Deck Builder")
                                    .size(15.0)
                                    .color(Color32::from_rgb(142, 203, 240)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if build.clicked() {
                            *next_scene = Some(Scene::DeckBuilder(
                                crate::scene::deck_builder::DeckBuilder::from_menu(
                                    self.client.clone(),
                                    self.player_id,
                                    self.player_name.clone(),
                                    self.available_decks.clone(),
                                    self.saved_decks.clone(),
                                    self.collection.clone(),
                                ),
                            ));
                        }
                        if !self.unopened_booster_packs.is_empty() {
                            ui.add_space(18.0);
                            let packs = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!(
                                        "✦ {} unopened pack{}",
                                        self.unopened_booster_packs.len(),
                                        if self.unopened_booster_packs.len() == 1 { "" } else { "s" }
                                    ))
                                    .size(14.0)
                                    .color(MENU_GOLD),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if packs.clicked() {
                                self.show_packs = true;
                            }
                        }
                    });
                });
                ui.add_space(10.0);

                let saved = self.saved_decks.clone();
                if saved.is_empty() {
                    egui::Frame::new()
                        .fill(Color32::from_rgba_premultiplied(21, 28, 27, 235))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::same(18))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("No decks in your collection yet.")
                                    .color(Color32::from_rgb(150, 165, 195))
                                    .size(14.0),
                            );
                        });
                    return;
                }

                if !matches!(self.selected_saved_deck, Some(index) if index < saved.len()) {
                    self.selected_saved_deck = Some(0);
                }

                if saved.len() <= 3 {
                    for (idx, deck_list) in saved.iter().enumerate() {
                        self.render_saved_deck_row(ui, idx, deck_list.clone(), next_scene);
                        ui.add_space(6.0);
                    }
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("saved_decks")
                        .max_height(340.0)
                        .min_scrolled_height(286.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                        for (idx, deck_list) in saved.iter().enumerate() {
                            self.render_saved_deck_row(ui, idx, deck_list.clone(), next_scene);
                            ui.add_space(6.0);
                        }
                    });
                }

                let selected_deck = self
                    .selected_saved_deck
                    .and_then(|index| saved.get(index))
                    .cloned();
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let play_label = selected_deck
                            .as_ref()
                            .map(|deck| format!("▶ Play {}", deck.name))
                            .unwrap_or_else(|| "▶ Play selected deck".to_string());
                        let play = ui.add_enabled(
                            selected_deck.is_some(),
                            egui::Button::new(
                                egui::RichText::new(play_label)
                                    .size(16.0)
                                    .color(Color32::WHITE),
                            )
                            .min_size(vec2(250.0, theme::BUTTON_HEIGHT)),
                        );
                        if play.clicked() && let Some(deck) = selected_deck {
                            self.play_custom_deck(deck);
                        }
                });
            });
    }

    fn render_saved_deck_row(
        &mut self,
        ui: &mut Ui,
        idx: usize,
        deck_list: DeckList,
        next_scene: &mut Option<Scene>,
    ) {
        let selected = self.selected_saved_deck == Some(idx);
        let fill = if selected {
            Color32::from_rgb(31, 42, 43)
        } else {
            Color32::from_rgb(18, 24, 25)
        };
        let border = if selected { theme::SELECTION } else { MENU_BORDER };
        let row = egui::Frame::new()
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, border))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let name_w = (ui.available_width() - 110.0).max(220.0);
                    ui.vertical(|ui| {
                        ui.set_width(name_w);
                        ui.label(
                            egui::RichText::new(&deck_list.name)
                                .color(Color32::from_rgb(225, 235, 255))
                                .size(16.0)
                                .strong(),
                        );
                        let site_count: u32 = deck_list.sites.iter().map(|c| c.count as u32).sum();
                        let spell_count: u32 =
                            deck_list.spells.iter().map(|c| c.count as u32).sum();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} spells · {} sites",
                                deck_list.avatar, spell_count, site_count
                            ))
                            .color(Color32::from_rgb(130, 145, 180))
                            .size(12.0),
                        );
                        let foil_cards = self.foil_cards_in_deck(&deck_list);
                        if foil_cards > 0 {
                            ui.label(
                                egui::RichText::new(format!("✦ {foil_cards} foil card(s)"))
                                    .color(Color32::from_rgb(255, 215, 120))
                                    .size(12.0),
                            );
                        }
                        if self
                            .available_decks
                            .iter()
                            .any(|starter| deck_list.name == format!("{} Precon", starter.name()))
                        {
                            ui.label(
                                egui::RichText::new("Preconstructed starter deck")
                                    .color(Color32::from_rgb(255, 200, 80))
                                    .size(12.0),
                            );
                        }
                    });

                    let edit_btn = egui::Button::new(
                        egui::RichText::new("✏ Edit")
                            .size(14.0)
                            .color(Color32::from_rgb(210, 225, 255)),
                    )
                    .min_size(vec2(92.0, 34.0));
                    if ui.add(edit_btn).clicked() {
                        self.selected_saved_deck = Some(idx);
                        *next_scene = Some(Scene::DeckBuilder(
                            crate::scene::deck_builder::DeckBuilder::from_deck_list(
                                self.client.clone(),
                                self.player_id,
                                self.player_name.clone(),
                                self.available_decks.clone(),
                                self.saved_decks.clone(),
                                self.collection.clone(),
                                deck_list,
                            ),
                        ));
                    }
                });
            });
        let select_response = ui.interact(
            row.response.rect,
            ui.id().with(("saved_deck_row", idx)),
            egui::Sense::click(),
        );
        if select_response.clicked() {
            self.selected_saved_deck = Some(idx);
        }
    }

    fn render_opened_booster_pack(&mut self, ui: &mut Ui) {
        let Some(pack) = self.opened_booster_pack.clone() else {
            return;
        };
        let tray_width = ui.available_width().min(1_200.0);
        let left_padding = ((ui.available_width() - tray_width) / 2.0).max(0.0);

        ui.add_space(36.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(format!("{} Booster Opened", pack.set_name))
                    .color(MENU_GOLD)
                    .font(theme::display_bold_font(38.0)),
            );
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("All cards in this pack have been added to your collection.")
                    .color(MENU_TEXT_MUTED)
                    .size(15.0),
            );
        });
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            ui.add_space(left_padding);
            ui.vertical(|ui| {
                ui.set_width(tray_width);
                egui::Frame::new()
                    .fill(theme::PANEL_BG)
                    .stroke(egui::Stroke::new(1.0, MENU_BORDER))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        egui::Grid::new("opened_booster_pack_grid")
                            .num_columns(5)
                            .spacing(vec2(14.0, 14.0))
                            .show(ui, |ui| {
                                for (index, card) in pack.cards.iter().enumerate() {
                                    Self::render_reward_card(ui, card, vec2(218.0, 266.0));
                                    if index % 5 == 4 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            });
        });
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            let continue_clicked = ui
                .add(
                    egui::Button::new(egui::RichText::new("Continue").size(16.0))
                        .min_size(vec2(136.0, theme::BUTTON_HEIGHT)),
                )
                .clicked()
                || ui.ctx().input(|input| input.key_pressed(egui::Key::Enter));
            if continue_clicked {
                self.opened_booster_pack = None;
            }
        });
        ui.add_space(28.0);
    }

    fn render_reward_card(ui: &mut Ui, booster_card: &BoosterCard, size: egui::Vec2) {
        let card_name = &booster_card.name;
        let card = Self::card_preview_data(card_name);
        let response =
            ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Center), |ui| {
                if let Some(texture) = TextureCache::get_card_texture_blocking(&card, ui.ctx()) {
                    let image = ui.add(
                        egui::Image::new(egui::ImageSource::Texture(
                            egui::load::SizedTexture::from_handle(&texture),
                        ))
                        .max_size(size),
                    );
                    if booster_card.is_foil {
                        Self::paint_foil_effect(ui, image.rect);
                    }
                } else {
                    ui.ctx().request_repaint();
                    ui.allocate_space(size);
                }
            });
        response.response.on_hover_ui(|ui| {
            if let Some(texture) = TextureCache::get_card_texture_blocking(&card, ui.ctx()) {
                let preview = ui.add(
                    egui::Image::new(egui::ImageSource::Texture(
                        egui::load::SizedTexture::from_handle(&texture),
                    ))
                    .max_size(vec2(420.0, 554.0)),
                );
                if booster_card.is_foil {
                    Self::paint_foil_effect(ui, preview.rect);
                }
            } else {
                ui.ctx().request_repaint();
                ui.allocate_space(vec2(420.0, 554.0));
            }
        });
    }

    fn paint_foil_effect(ui: &Ui, rect: egui::Rect) {
        let painter = ui.painter().with_clip_rect(rect);

        // Real foil keeps the ink dark and only shifts the reflected colour.
        // The spectrum is fixed to the card surface because the card itself does
        // not tilt or move in this view.
        let colors = [
            Color32::from_rgba_unmultiplied(255, 55, 105, 67),
            Color32::from_rgba_unmultiplied(255, 175, 45, 58),
            Color32::from_rgba_unmultiplied(80, 245, 145, 62),
            Color32::from_rgba_unmultiplied(45, 190, 255, 72),
            Color32::from_rgba_unmultiplied(105, 75, 255, 66),
            Color32::from_rgba_unmultiplied(235, 65, 250, 64),
        ];
        let band_width = rect.width() / 5.0;
        let slant = rect.height() * 0.16;
        let first_x = rect.left() - slant - band_width * 0.65;
        let mut mesh = egui::Mesh::default();
        let column_count = 9u32;

        for column in 0..=column_count {
            let x = first_x + column as f32 * band_width;
            let color = colors[column as usize % colors.len()];
            mesh.colored_vertex(egui::pos2(x + slant, rect.top()), color);
            mesh.colored_vertex(egui::pos2(x - slant, rect.bottom()), color);
        }
        for column in 0..column_count {
            let top_left = column * 2;
            let bottom_left = top_left + 1;
            let top_right = top_left + 2;
            let bottom_right = top_left + 3;
            mesh.add_triangle(top_left, bottom_left, top_right);
            mesh.add_triangle(top_right, bottom_left, bottom_right);
        }
        painter.add(egui::Shape::mesh(mesh));

        // A fixed, feathered highlight gives the foil a reflective focal point.
        let sweep = rect.width() * 0.08 - rect.height() * 0.28;
        let glow_colors = [
            Color32::TRANSPARENT,
            Color32::from_rgba_unmultiplied(210, 235, 255, 24),
            Color32::from_rgba_unmultiplied(245, 252, 255, 55),
            Color32::from_rgba_unmultiplied(215, 238, 255, 28),
            Color32::TRANSPARENT,
        ];
        let glow_offsets = [-34.0, -18.0, 0.0, 18.0, 34.0];
        let mut glow = egui::Mesh::default();
        for (offset, color) in glow_offsets.into_iter().zip(glow_colors) {
            glow.colored_vertex(
                egui::pos2(
                    rect.left() + sweep + rect.height() * 0.45 + offset,
                    rect.top(),
                ),
                color,
            );
            glow.colored_vertex(
                egui::pos2(rect.left() + sweep + offset, rect.bottom()),
                color,
            );
        }
        for column in 0..4u32 {
            let top_left = column * 2;
            let bottom_left = top_left + 1;
            let top_right = top_left + 2;
            let bottom_right = top_left + 3;
            glow.add_triangle(top_left, bottom_left, top_right);
            glow.add_triangle(top_right, bottom_left, bottom_right);
        }
        painter.add(egui::Shape::mesh(glow));
        painter.rect_stroke(
            rect.shrink(0.5),
            3.0,
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(205, 235, 245, 185)),
            egui::StrokeKind::Inside,
        );
    }

    fn render_packs(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("Your Booster Packs")
                    .color(Color32::from_rgb(255, 200, 60))
                    .font(theme::display_bold_font(38.0)),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} unopened {} waiting at your table",
                    self.unopened_booster_packs.len(),
                    if self.unopened_booster_packs.len() == 1 {
                        "pack is"
                    } else {
                        "packs are"
                    }
                ))
                .color(MENU_TEXT_MUTED)
                .size(15.0),
            );
            ui.add_space(20.0);

            ui.label(
                egui::RichText::new("Choose a pack to open it.")
                    .color(MENU_TEXT)
                    .size(16.0),
            );
            ui.add_space(12.0);
            let packs = self.unopened_booster_packs.clone();
            let pack_size = vec2(170.0, 220.0);
            let overlap = 58.0;
            let stack_width = pack_size.x + overlap * packs.len().saturating_sub(1) as f32;
            egui::ScrollArea::horizontal()
                .id_salt("owned_booster_packs")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let (stack_rect, _) = ui.allocate_exact_size(
                        vec2(stack_width, pack_size.y + 28.0),
                        egui::Sense::hover(),
                    );
                    let mut hovered_index = None;
                    let mut lifts = Vec::with_capacity(packs.len());
                    for (index, pack) in packs.iter().enumerate() {
                        let rect = egui::Rect::from_min_size(
                            stack_rect.min + vec2(index as f32 * overlap, 0.0),
                            pack_size,
                        );
                        let response = ui.interact(
                            rect,
                            ui.id().with(("owned_booster_pack", pack.id)),
                            egui::Sense::click(),
                        );
                        if response.hovered() {
                            hovered_index = Some(index);
                        }
                        lifts.push(ui.ctx().animate_bool_with_time_and_easing(
                            ui.id().with(("booster_pack_lift", pack.id)),
                            response.hovered(),
                            theme::animation_time(ui.ctx(), 0.18),
                            egui::emath::easing::cubic_out,
                        ));
                        if response.clicked() {
                            self.client
                                .send(ClientMessage::OpenBoosterPack { pack_id: pack.id })
                                .ok();
                        }
                    }
                    let draw_pack = |ui: &mut Ui, rect: egui::Rect| {
                        if let Some(texture) = TextureCache::get_texture_blocking(
                            "assets/images/beta_booster_1.webp",
                            ui.ctx(),
                        ) {
                            egui::Image::new(egui::ImageSource::Texture(
                                egui::load::SizedTexture::from_handle(&texture),
                            ))
                            .paint_at(ui, rect);
                        } else {
                            ui.ctx().request_repaint();
                        }
                    };
                    for index in 0..packs.len() {
                        if lifts[index] <= f32::EPSILON {
                            draw_pack(
                                ui,
                                egui::Rect::from_min_size(
                                    stack_rect.min + vec2(index as f32 * overlap, 0.0),
                                    pack_size,
                                ),
                            );
                        }
                    }
                    for index in 0..packs.len() {
                        if lifts[index] > f32::EPSILON && Some(index) != hovered_index {
                            let lift = lifts[index];
                            let lifted_rect = egui::Rect::from_min_size(
                                stack_rect.min
                                    + vec2(index as f32 * overlap - 7.0 * lift, -9.0 * lift),
                                pack_size + vec2(14.0 * lift, 18.0 * lift),
                            );
                            draw_pack(ui, lifted_rect);
                        }
                    }
                    if let Some(index) = hovered_index {
                        let lift = lifts[index];
                        let lifted_rect = egui::Rect::from_min_size(
                            stack_rect.min
                                + vec2(index as f32 * overlap - 7.0 * lift, -9.0 * lift),
                            pack_size + vec2(14.0 * lift, 18.0 * lift),
                        );
                        draw_pack(ui, lifted_rect);
                        ui.painter().text(
                            stack_rect.center_bottom() - vec2(0.0, 10.0),
                            egui::Align2::CENTER_CENTER,
                            "Open pack",
                            egui::FontId::proportional(13.0),
                            MENU_GOLD,
                        );
                    }
                });
            ui.add_space(18.0);
            if ui.button("Back").clicked() {
                self.show_packs = false;
            }
        });
    }

    fn card_preview_data(name: &str) -> CardData {
        let card = from_name(name, &uuid::Uuid::nil());
        let base = card.get_base();
        CardData {
            id: uuid::Uuid::nil(),
            name: card.get_name().to_string(),
            owner_id: PlayerId::nil(),
            controller_id: PlayerId::nil(),
            zone_sequence: 0,
            tapped: false,
            edition: base.edition.clone(),
            zone: base.zone.clone(),
            region: Region::Surface,
            card_type: card.get_card_type(),
            abilities: vec![],
            statuses: vec![],
            damage_taken: 0,
            bearer: None,
            rarity: base.rarity.clone(),
            power: card
                .get_unit_base()
                .map(|unit| unit.power)
                .unwrap_or_default(),
            has_attachments: false,
            image_path: card.get_image_path(),
            is_token: base.is_token,
        }
    }

    pub fn process_message(&mut self, msg: &ServerMessage) -> Option<Scene> {
        match msg {
            ServerMessage::ConnectResponse {
                player_id,
                available_decks,
            } => {
                self.available_decks = available_decks.clone();
                self.player_id = Some(*player_id);
                self.connect_requested = false;
                None
            }
            ServerMessage::AuthenticationSuccess {
                player_id,
                username,
                available_decks,
                saved_decks,
                collection,
                unopened_booster_packs,
                reward_points,
            } => {
                self.available_decks = available_decks.clone();
                self.saved_decks = saved_decks.clone();
                self.collection = collection.clone();
                self.player_id = Some(*player_id);
                self.player_name = username.clone();
                self.password.clear();
                self.confirmation_code.clear();
                self.auth_requested = false;
                self.auth_error = None;
                self.awaiting_email_confirmation = false;
                self.unopened_booster_packs = unopened_booster_packs.clone();
                self.reward_points = *reward_points;
                self.booster_reward = (!unopened_booster_packs.is_empty()).then(|| {
                    format!(
                        "Weekly reward: {} unopened Beta booster packs.",
                        unopened_booster_packs.len()
                    )
                });
                self.selecting_starter_deck = false;
                None
            }
            ServerMessage::AuthenticationFailure { message } => {
                self.auth_requested = false;
                self.auth_error = Some(message.clone());
                None
            }
            ServerMessage::EmailConfirmationRequired {
                email,
                delivery_failed,
            } => {
                self.email = email.clone();
                self.awaiting_email_confirmation = true;
                self.auth_requested = false;
                self.auth_error = delivery_failed.then(|| {
                    "We could not send a code. Check your email details and try resending."
                        .to_string()
                });
                None
            }
            ServerMessage::StarterDeckSelection {
                username,
                available_decks,
            } => {
                self.player_name = username.clone();
                self.password.clear();
                self.auth_requested = false;
                self.selecting_starter_deck = true;
                self.starter_decks = available_decks.clone();
                None
            }
            ServerMessage::BoosterPackOpened { pack_id, pack } => {
                self.unopened_booster_packs
                    .retain(|unopened| unopened.id != *pack_id);
                for booster_card in &pack.cards {
                    if let Some(card) = self.collection.iter_mut().find(|card| {
                        card.name == booster_card.name && card.is_foil == booster_card.is_foil
                    }) {
                        card.count = card.count.saturating_add(1);
                    } else {
                        self.collection.push(CollectedCard {
                            name: booster_card.name.clone(),
                            count: 1,
                            is_foil: booster_card.is_foil,
                        });
                    }
                }
                self.opened_booster_pack = Some(pack.clone());
                None
            }
            ServerMessage::BoosterRedeemed { reward_points, pack } => {
                self.reward_points = *reward_points;
                self.unopened_booster_packs.push(pack.clone());
                self.reward_redemption_requested = false;
                self.reward_feedback = Some("Beta Booster added to your packs.".to_string());
                None
            }
            ServerMessage::RewardRedemptionFailed { message } => {
                self.reward_redemption_requested = false;
                self.reward_feedback = Some(message.clone());
                None
            }
            ServerMessage::GameStarted {
                player1,
                player2,
                game_id,
                cards,
            } => {
                let player_id = self.player_id?;
                let opponent_id = if player1 == &player_id {
                    *player2
                } else {
                    *player1
                };

                let mut manager =
                    AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).ok()?;
                if let Ok(sound_data) = StaticSoundData::from_file("assets/sounds/game_start.mp3") {
                    manager.play(sound_data).ok();
                }

                Some(Scene::Game(Game::new(
                    *game_id,
                    player_id,
                    opponent_id,
                    player1 == &player_id,
                    cards.clone(),
                    self.client.clone(),
                    manager,
                    Menu::restore(
                        self.client.clone(),
                        self.player_id,
                        self.player_name.clone(),
                        self.available_decks.clone(),
                        self.saved_decks.clone(),
                        self.collection.clone(),
                    ),
                    self.reward_points,
                )))
            }
            _ => None,
        }
    }

    pub fn render(&mut self, ui: &mut Ui) -> Option<Scene> {
        let time = ui.ctx().input(|i| i.time);

        #[cfg(feature = "name-entry")]
        let shake_x: f32 = if let Some(start) = self.shake_start {
            let elapsed = (time - start) as f32;
            if elapsed < 0.45 {
                ui.ctx().request_repaint();
                let amplitude = 11.0 * (1.0 - elapsed / 0.45);
                (elapsed * 38.0).sin() * amplitude
            } else {
                self.shake_start = None;
                0.0
            }
        } else {
            0.0
        };

        // Clear error state once the user has typed something
        if !self.player_name.is_empty() {
            self.show_name_error = false;
        }

        let mut next_scene: Option<Scene> = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(MENU_BG))
            .show_inside(ui, |ui| {
                self.render_menu_background(ui);
                let panel_h = ui.available_height();
                if self.opened_booster_pack.is_some() {
                    self.render_opened_booster_pack(ui);
                    return;
                }
                if self.show_packs {
                    self.render_packs(ui);
                    return;
                }
                if self.show_rewards {
                    self.render_rewards_screen(ui);
                    return;
                }
                let deck_selection_visible =
                    !self.available_decks.is_empty() && !self.looking_for_match;
                if deck_selection_visible {
                    self.render_reward_balance(ui);
                    egui::ScrollArea::vertical()
                        .id_salt("deck_selection_screen")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(24.0);
                            ui.vertical_centered(|ui| {
                                self.render_brand_heading(ui, true);
                                ui.add_space(14.0);

                                self.render_deck_selection(ui, &mut next_scene);
                            });
                            ui.add_space(24.0);
                        });
                    return;
                }

                ui.add_space(panel_h * 0.18);

                ui.vertical_centered(|ui| {
                    self.render_brand_heading(ui, false);
                    ui.add_space(28.0);

                    if self.looking_for_match {
                        let dot_count = ((time * 2.0) as usize % 3) + 1;
                        let dots = ".".repeat(dot_count) + &" ".repeat(3 - dot_count);
                        ui.label(
                            egui::RichText::new(format!("Looking for match{dots}"))
                                .color(Color32::WHITE)
                                .size(28.0),
                        );
                    } else if self.selecting_starter_deck {
                        ui.label(
                            egui::RichText::new("Choose your starter deck")
                                .color(MENU_TEXT)
                                .size(22.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                "Its cards will be added to your collection and ready for your first match.",
                            )
                            .color(MENU_TEXT_MUTED)
                            .size(15.0),
                        );
                        ui.add_space(18.0);
                        for deck in self.starter_decks.clone() {
                            if ui
                                .add_enabled(
                                    !self.auth_requested,
                                    egui::Button::new(deck.name())
                                        .min_size(vec2(300.0, theme::BUTTON_HEIGHT)),
                                )
                                .clicked()
                            {
                                if self
                                    .client
                                    .send(ClientMessage::ChooseStarterDeck { deck })
                                    .is_ok()
                                {
                                    self.auth_requested = true;
                                } else {
                                    self.auth_error = Some("Unable to reach the server".to_string());
                                }
                            }
                            ui.add_space(6.0);
                        }
                    } else if self.available_decks.is_empty() {
                        self.render_auth_card(ui);
                    } else {
                        self.render_deck_selection(ui, &mut next_scene);
                    }
                });
            });

        next_scene
    }
}
