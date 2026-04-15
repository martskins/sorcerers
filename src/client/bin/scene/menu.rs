use crate::scene::{Scene, game::Game};
use egui::{Color32, Context, Frame, Margin, Ui, vec2};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use sorcerers::deck::DeckList;
use sorcerers::game::PlayerId;
use sorcerers::networking::message::ServerMessage;
use sorcerers::networking::{
    self,
    message::{ClientMessage, DeckChoice, PreconDeck},
};

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
                        let (base_rect, _) = ui.allocate_exact_size(vec2(input_w, input_h), egui::Sense::hover());
                        let shaken_rect = base_rect.translate(vec2(shake_x, 0.0));

                        // Inset the TextEdit slightly so it doesn't paint over our border
                        let inner = shaken_rect.shrink(4.0);
                        let te = egui::TextEdit::singleline(&mut self.player_name)
                            .font(egui::FontId::proportional(24.0))
                            .text_color(Color32::DARK_GRAY)
                            .hint_text("Your name…")
                            .background_color(Color32::LIGHT_GRAY)
                            .margin(Margin::same(5))
                            .frame(Frame::NONE); // we draw our own frame above
                        let resp = ui.put(inner, te);

                        // Auto-focus the field on first render
                        if resp.gained_focus() || (!resp.has_focus() && self.player_name.is_empty()) {
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

                        let btn =
                            egui::Button::new(egui::RichText::new("Search for Match").size(24.0).color(Color32::WHITE))
                                .min_size(vec2(280.0, 52.0));

                        let clicked = ui.add(btn).clicked() || ui.ctx().input(|i| i.key_pressed(egui::Key::Enter));

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
                        // ── Deck selection ────────────────────────────────────
                        ui.label(
                            egui::RichText::new("Choose a deck")
                                .color(Color32::from_rgb(180, 180, 220))
                                .size(26.0),
                        );
                        ui.add_space(16.0);

                        // Precon decks
                        for deck in self.available_decks.clone() {
                            let btn =
                                egui::Button::new(egui::RichText::new(deck.name()).size(22.0).color(Color32::WHITE))
                                    .min_size(vec2(280.0, 50.0));
                            if ui.add(btn).clicked() {
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
                            ui.add_space(10.0);
                        }

                        // Saved custom decks — dropdown + play button
                        let saved = self.saved_decks.clone();
                        if !saved.is_empty() {
                            ui.add_space(16.0);
                            ui.label(
                                egui::RichText::new("Custom Deck")
                                    .color(Color32::from_rgb(140, 160, 200))
                                    .size(16.0),
                            );
                            ui.add_space(6.0);

                            let selected_label = self
                                .selected_saved_deck
                                .and_then(|i| saved.get(i))
                                .map(|d| d.name.as_str())
                                .unwrap_or("— select a deck —");

                            // ComboBox doesn't respond to vertical_centered, so center manually
                            let combo_w = 280.0_f32;
                            let avail_w = ui.available_width();
                            let padding = ((avail_w - combo_w) / 2.0).max(0.0);
                            ui.horizontal(|ui| {
                                ui.add_space(padding);
                                egui::ComboBox::from_id_salt("saved_deck_combo")
                                    .selected_text(
                                        egui::RichText::new(selected_label)
                                            .color(Color32::from_rgb(200, 230, 255))
                                            .size(16.0),
                                    )
                                    .width(combo_w)
                                    .show_ui(ui, |ui| {
                                        for (i, deck_list) in saved.iter().enumerate() {
                                            let is_sel = self.selected_saved_deck == Some(i);
                                            ui.selectable_value(
                                                &mut self.selected_saved_deck,
                                                Some(i),
                                                egui::RichText::new(&deck_list.name)
                                                    .color(if is_sel {
                                                        Color32::from_rgb(255, 210, 80)
                                                    } else {
                                                        Color32::from_rgb(200, 230, 255)
                                                    })
                                                    .size(15.0),
                                            );
                                        }
                                    });
                            });

                            ui.add_space(8.0);

                            let can_play = self.selected_saved_deck.is_some();

                            // Play + Edit on the same row
                            let btn_w = 136.0_f32;
                            let gap = 8.0_f32;
                            let total_w = btn_w * 2.0 + gap;
                            let avail_w2 = ui.available_width();
                            let left_pad = ((avail_w2 - total_w) / 2.0).max(0.0);

                            ui.horizontal(|ui| {
                                ui.add_space(left_pad);

                                let play_btn =
                                    egui::Button::new(egui::RichText::new("▶ Play").size(17.0).color(if can_play {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(100, 110, 140)
                                    }))
                                    .min_size(vec2(btn_w, 42.0));
                                if ui.add_enabled(can_play, play_btn).clicked() {
                                    if let Some(idx) = self.selected_saved_deck {
                                        if let Some(deck_list) = saved.get(idx).cloned() {
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
                                    }
                                }

                                ui.add_space(gap);

                                let edit_btn =
                                    egui::Button::new(egui::RichText::new("✏ Edit").size(17.0).color(if can_play {
                                        Color32::from_rgb(200, 220, 255)
                                    } else {
                                        Color32::from_rgb(100, 110, 140)
                                    }))
                                    .min_size(vec2(btn_w, 42.0));
                                if ui.add_enabled(can_play, edit_btn).clicked() {
                                    if let Some(idx) = self.selected_saved_deck {
                                        if let Some(deck_list) = saved.get(idx).cloned() {
                                            next_scene = Some(Scene::DeckBuilder(
                                                crate::scene::deck_builder::DeckBuilder::from_deck_list(
                                                    self.client.clone(),
                                                    self.player_id,
                                                    self.player_name.clone(),
                                                    self.available_decks.clone(),
                                                    deck_list,
                                                ),
                                            ));
                                        }
                                    }
                                }
                            });
                        }

                        // Deck error message
                        if let Some(ref err) = self.deck_error.clone() {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("⚠ {err}"))
                                    .color(Color32::from_rgb(220, 80, 60))
                                    .size(14.0),
                            );
                            ui.add_space(4.0);
                        }

                        ui.add_space(8.0);
                        let build_btn = egui::Button::new(
                            egui::RichText::new("🔨 Deck Builder")
                                .size(20.0)
                                .color(Color32::from_rgb(200, 220, 255)),
                        )
                        .min_size(vec2(280.0, 46.0));
                        if ui.add(build_btn).clicked() {
                            next_scene = Some(Scene::DeckBuilder(crate::scene::deck_builder::DeckBuilder::from_menu(
                                self.client.clone(),
                                self.player_id,
                                self.player_name.clone(),
                                self.available_decks.clone(),
                            )));
                        }
                    }
                });
            });

        next_scene
    }
}
