use crate::scene::{Scene, game::Game};
use egui::{Color32, Context, Frame, Ui, vec2};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use sorcerers::deck::DeckList;
use sorcerers::deck::precon::PreconDeck;
use sorcerers::game::PlayerId;
use sorcerers::networking::message::ServerMessage;
use sorcerers::networking::{
    self,
    message::{ClientMessage, DeckChoice},
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Menu {
    client: networking::client::Client,
    player_id: Option<PlayerId>,
    available_decks: Vec<PreconDeck>,
    saved_decks: Vec<DeckList>,
    selected_saved_deck: Option<usize>,
    deck_error: Option<String>,
    looking_for_match: bool,
    player_name: String,
    /// Time (seconds, from `ctx.input`) when the shake was triggered.
    shake_start: Option<f64>,
    /// True while the name field is empty after a failed submit attempt.
    show_name_error: bool,
}

impl Menu {
    pub fn new(client: networking::client::Client) -> Self {
        Self {
            client,
            player_id: None,
            available_decks: vec![],
            saved_decks: DeckList::load_all(),
            selected_saved_deck: None,
            deck_error: None,
            looking_for_match: false,
            player_name: String::new(),
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
    ) -> Self {
        Self {
            client,
            player_id,
            available_decks,
            saved_decks: DeckList::load_all(),
            selected_saved_deck: None,
            deck_error: None,
            looking_for_match: false,
            player_name,
            shake_start: None,
            show_name_error: false,
        }
    }

    pub fn update(&mut self, _ctx: &Context) {}

    fn precon_parts(deck: &PreconDeck) -> (&'static str, &'static str) {
        deck.name()
            .split_once(" - ")
            .unwrap_or(("Precon", deck.name()))
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

    fn render_deck_selection(&mut self, ui: &mut Ui, next_scene: &mut Option<Scene>) {
        ui.label(
            egui::RichText::new("Choose a deck")
                .color(Color32::from_rgb(220, 222, 245))
                .size(28.0),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Pick a precon for a quick game, or play and edit saved decks.")
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

                let precon_h = 330.0;
                self.render_precon_section(ui, content_w, precon_h);
                ui.add_space(14.0);
                // The precon section uses a 10 pixel margin between each button column, so we give
                // it 10 pixels more width to make sure both have the same total width.
                self.render_custom_section(ui, next_scene, content_w + 10.0, 330.0);

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

    fn render_precon_section(&mut self, ui: &mut Ui, width: f32, height: f32) {
        egui::Frame::new()
            .fill(Color32::from_rgb(15, 18, 30))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(42, 52, 76)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(width - 28.0);
                ui.set_min_height(height - 28.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Preconstructed")
                            .color(Color32::from_rgb(235, 238, 255))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{} decks", self.available_decks.len()))
                            .color(Color32::from_rgb(125, 145, 180))
                            .size(13.0),
                    );
                });
                ui.add_space(10.0);

                let mut grouped: BTreeMap<&'static str, Vec<PreconDeck>> = BTreeMap::new();
                for deck in self.available_decks.clone() {
                    let (set, _) = Self::precon_parts(&deck);
                    grouped.entry(set).or_default().push(deck);
                }

                egui::ScrollArea::vertical()
                    .id_salt("precon_decks")
                    .max_height(height - 56.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (set, decks) in grouped {
                            ui.label(
                                egui::RichText::new(set)
                                    .color(Color32::from_rgb(255, 200, 80))
                                    .size(14.0)
                                    .strong(),
                            );
                            ui.add_space(6.0);

                            let gap = 10.0;
                            let card_w = ((ui.available_width() - gap) / 2.0).max(180.0);
                            for row in decks.chunks(2) {
                                ui.horizontal(|ui| {
                                    for (idx, deck) in row.iter().enumerate() {
                                        self.render_precon_button(ui, deck.clone(), card_w);
                                        if idx + 1 < row.len() {
                                            ui.add_space(gap);
                                        }
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        }
                    });
            });
    }

    fn render_precon_button(&mut self, ui: &mut Ui, deck: PreconDeck, width: f32) {
        let (_, element) = Self::precon_parts(&deck);
        let accent = match element {
            "Earth" => Color32::from_rgb(117, 176, 96),
            "Water" => Color32::from_rgb(74, 154, 210),
            "Air" => Color32::from_rgb(180, 195, 230),
            "Fire" => Color32::from_rgb(224, 107, 72),
            _ => Color32::from_rgb(180, 160, 220),
        };

        let button = egui::Button::new(
            egui::RichText::new(element)
                .size(19.0)
                .color(Color32::WHITE)
                .strong(),
        )
        .fill(Color32::from_rgb(35, 44, 65))
        .stroke(egui::Stroke::new(1.5, accent))
        .min_size(vec2(width, 48.0));

        if ui.add(button).on_hover_text(deck.name()).clicked() {
            self.play_precon(deck);
        }
    }

    fn render_custom_section(
        &mut self,
        ui: &mut Ui,
        next_scene: &mut Option<Scene>,
        width: f32,
        height: f32,
    ) {
        egui::Frame::new()
            .fill(Color32::from_rgb(13, 16, 27))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(42, 52, 76)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(width - 28.0);
                ui.set_min_height(height - 28.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Custom Decks")
                            .color(Color32::from_rgb(235, 238, 255))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{} saved", self.saved_decks.len()))
                            .color(Color32::from_rgb(125, 145, 180))
                            .size(13.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let build_btn = egui::Button::new(
                            egui::RichText::new("🔨 Deck Builder")
                                .size(15.0)
                                .color(Color32::from_rgb(215, 228, 255)),
                        )
                        .min_size(vec2(154.0, 34.0));
                        if ui.add(build_btn).clicked() {
                            *next_scene = Some(Scene::DeckBuilder(
                                crate::scene::deck_builder::DeckBuilder::from_menu(
                                    self.client.clone(),
                                    self.player_id,
                                    self.player_name.clone(),
                                    self.available_decks.clone(),
                                ),
                            ));
                        }
                    });
                });
                ui.add_space(10.0);

                let saved = self.saved_decks.clone();
                if saved.is_empty() {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(20, 24, 38))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::same(18))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(
                                    "No saved custom decks yet. Build one when you want to tune your own list.",
                                )
                                .color(Color32::from_rgb(150, 165, 195))
                                .size(14.0),
                            );
                        });
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_salt("saved_decks")
                    .max_height(height - 76.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, deck_list) in saved.iter().enumerate() {
                            self.render_saved_deck_row(ui, idx, deck_list.clone(), next_scene);
                            ui.add_space(6.0);
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
            Color32::from_rgb(36, 45, 66)
        } else {
            Color32::from_rgb(20, 24, 38)
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(42, 52, 76)))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let name_w = (ui.available_width() - 210.0).max(220.0);
                    ui.vertical(|ui| {
                        ui.set_width(name_w);
                        let name_response = ui.selectable_label(
                            selected,
                            egui::RichText::new(&deck_list.name)
                                .color(Color32::from_rgb(225, 235, 255))
                                .size(16.0)
                                .strong(),
                        );
                        if name_response.clicked() {
                            self.selected_saved_deck = Some(idx);
                        }
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
                    });

                    let play_btn = egui::Button::new(
                        egui::RichText::new("▶ Play")
                            .size(14.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(vec2(92.0, 34.0));
                    if ui.add(play_btn).clicked() {
                        self.selected_saved_deck = Some(idx);
                        self.play_custom_deck(deck_list.clone());
                    }

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
                                deck_list,
                            ),
                        ));
                    }
                });
            });
    }

    pub fn process_message(&mut self, msg: &ServerMessage) -> Option<Scene> {
        match msg {
            ServerMessage::ConnectResponse {
                player_id,
                available_decks,
            } => {
                self.available_decks = available_decks.clone();
                self.player_id = Some(*player_id);
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
                )))
            }
            _ => None,
        }
    }

    pub fn render(&mut self, ui: &mut Ui) -> Option<Scene> {
        let time = ui.ctx().input(|i| i.time);

        // ── Shake calculation (decaying sine, 0.45 s duration) ───────────────
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
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(8, 8, 14)))
            .show_inside(ui, |ui| {
                let panel_h = ui.available_height();
                let deck_selection_visible =
                    !self.available_decks.is_empty() && !self.looking_for_match;
                if deck_selection_visible {
                    egui::ScrollArea::vertical()
                        .id_salt("deck_selection_screen")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(24.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("✦  Sorcerers  ✦")
                                        .color(Color32::from_rgb(255, 200, 60))
                                        .size(44.0)
                                        .strong(),
                                );
                                ui.add_space(14.0);

                                if self.client.is_in_local_mode() {
                                    ui.label(
                                        egui::RichText::new("⚠  Running in local mode")
                                            .color(Color32::from_rgb(255, 165, 0))
                                            .size(16.0),
                                    );
                                    ui.add_space(12.0);
                                }

                                self.render_deck_selection(ui, &mut next_scene);
                            });
                            ui.add_space(24.0);
                        });
                    return;
                }

                ui.add_space(panel_h * 0.18);

                ui.vertical_centered(|ui| {
                    // ── Title ─────────────────────────────────────────────────
                    ui.label(
                        egui::RichText::new("✦  Sorcerers  ✦")
                            .color(Color32::from_rgb(255, 200, 60))
                            .size(58.0)
                            .strong(),
                    );
                    ui.add_space(32.0);

                    if self.client.is_in_local_mode() {
                        ui.label(
                            egui::RichText::new("⚠  Running in local mode")
                                .color(Color32::from_rgb(255, 165, 0))
                                .size(16.0),
                        );
                        ui.add_space(12.0);
                    }

                    if self.looking_for_match {
                        let dot_count = ((time * 2.0) as usize % 3) + 1;
                        let dots = ".".repeat(dot_count) + &" ".repeat(3 - dot_count);
                        ui.label(
                            egui::RichText::new(format!("Looking for match{dots}"))
                                .color(Color32::WHITE)
                                .size(28.0),
                        );
                    } else if self.available_decks.is_empty() {
                        // ── Name entry ────────────────────────────────────────
                        ui.label(
                            egui::RichText::new("Enter your name")
                                .color(Color32::from_rgb(180, 180, 210))
                                .size(20.0),
                        );
                        ui.add_space(12.0);

                        // We render the input via `allocate_exact_size` +
                        // `ui.put` so we can apply the shake offset.
                        let input_w = 320.0;
                        let input_h = 46.0;
                        let (base_rect, _) =
                            ui.allocate_exact_size(vec2(input_w, input_h), egui::Sense::hover());
                        let shaken_rect = base_rect.translate(vec2(shake_x, 0.0));

                        let te = egui::TextEdit::singleline(&mut self.player_name)
                            .font(egui::FontId::proportional(24.0))
                            .text_color(Color32::DARK_GRAY)
                            .hint_text("Your name…")
                            .frame(
                                Frame::new()
                                    .corner_radius(2.5)
                                    .stroke(egui::Stroke::new(2.0, Color32::GRAY)),
                            );
                        let resp = ui.put(shaken_rect, te);

                        // Auto-focus the field on first render
                        if resp.gained_focus() || (!resp.has_focus() && self.player_name.is_empty())
                        {
                            resp.request_focus();
                        }

                        // Error hint text
                        let is_error = self.show_name_error && self.player_name.is_empty();
                        if is_error {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Please enter a name to continue")
                                    .color(Color32::from_rgb(220, 80, 60))
                                    .size(14.0),
                            );
                        } else {
                            ui.add_space(20.0); // reserve same space so layout doesn't shift
                        }

                        let btn = egui::Button::new(
                            egui::RichText::new("Search for Match")
                                .size(24.0)
                                .color(Color32::WHITE),
                        )
                        .min_size(vec2(280.0, 52.0));

                        let clicked = ui.add(btn).clicked()
                            || ui.ctx().input(|i| i.key_pressed(egui::Key::Enter));

                        if clicked {
                            if self.player_name.is_empty() {
                                // Trigger shake + error state
                                self.shake_start = Some(time);
                                self.show_name_error = true;
                                ui.ctx().request_repaint();
                            } else {
                                self.client.send(ClientMessage::Connect).ok();
                            }
                        }
                    } else {
                        self.render_deck_selection(ui, &mut next_scene);
                    }
                });
            });

        next_scene
    }
}
