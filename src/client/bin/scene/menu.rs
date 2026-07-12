use crate::scene::{Scene, game::Game};
use crate::texture_cache::TextureCache;
use egui::{Color32, Context, Ui, vec2};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use sorcerers::collection::CollectedCard;
use sorcerers::deck::DeckList;
use sorcerers::deck::precon::PreconDeck;
use sorcerers::game::PlayerId;
use sorcerers::booster::{BoosterCard, BoosterPack, UnopenedBoosterPack};
use sorcerers::card::{CardData, Region, from_name};
use sorcerers::networking::message::ServerMessage;
use sorcerers::networking::{
    self,
    message::{ClientMessage, DeckChoice},
};

#[derive(Debug)]
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
    password: String,
    registering: bool,
    auth_requested: bool,
    auth_error: Option<String>,
    booster_reward: Option<String>,
    unopened_booster_packs: Vec<UnopenedBoosterPack>,
    opened_booster_pack: Option<BoosterPack>,
    show_packs: bool,
    selecting_starter_deck: bool,
    starter_decks: Vec<PreconDeck>,
    connect_requested: bool,
    #[cfg(feature = "name-entry")]
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
            collection: vec![],
            selected_saved_deck: None,
            deck_error: None,
            looking_for_match: false,
            player_name: String::new(),
            username: String::new(),
            password: String::new(),
            registering: false,
            auth_requested: false,
            auth_error: None,
            booster_reward: None,
            unopened_booster_packs: vec![],
            opened_booster_pack: None,
            show_packs: false,
            selecting_starter_deck: false,
            starter_decks: vec![],
            connect_requested: false,
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
            password: String::new(),
            registering: false,
            auth_requested: false,
            auth_error: None,
            booster_reward: None,
            unopened_booster_packs: vec![],
            opened_booster_pack: None,
            show_packs: false,
            selecting_starter_deck: false,
            starter_decks: vec![],
            connect_requested: false,
            #[cfg(feature = "name-entry")]
            shake_start: None,
            show_name_error: false,
        }
    }

    pub fn update(&mut self, _ctx: &Context) {}

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
            egui::RichText::new("Choose a deck")
                .color(Color32::from_rgb(220, 222, 245))
                .size(28.0),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Play or edit a deck from your collection.")
                .color(Color32::from_rgb(130, 145, 180))
                .size(15.0),
        );
        if let Some(reward) = &self.booster_reward {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(reward)
                    .color(Color32::from_rgb(255, 200, 80))
                    .size(14.0),
            );
        }
        if !self.unopened_booster_packs.is_empty() {
            ui.add_space(10.0);
            if ui
                .add(
                    egui::Button::new(format!(
                        "Packs ({})",
                        self.unopened_booster_packs.len()
                    ))
                    .min_size(vec2(250.0, 36.0)),
            )
                .clicked()
            {
                self.show_packs = true;
            }
        }
        ui.add_space(18.0);

        let content_w = ui.available_width().min(860.0);
        let left_pad = ((ui.available_width() - content_w) / 2.0).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            ui.vertical(|ui| {
                ui.set_width(content_w);
                self.render_custom_section(ui, next_scene, content_w, 510.0);

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
                        egui::RichText::new("Your Decks")
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
                                    self.saved_decks.clone(),
                                    self.collection.clone(),
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
                                    "No decks in your collection yet.",
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
                        let foil_cards = self.foil_cards_in_deck(&deck_list);
                        if foil_cards > 0 {
                            ui.label(
                                egui::RichText::new(format!("✦ {foil_cards} foil card(s)"))
                                    .color(Color32::from_rgb(255, 215, 120))
                                    .size(12.0),
                            );
                        }
                        if self.available_decks.iter().any(|starter| {
                            deck_list.name == format!("{} Precon", starter.name())
                        }) {
                            ui.label(
                                egui::RichText::new("Preconstructed starter deck")
                                    .color(Color32::from_rgb(255, 200, 80))
                                    .size(12.0),
                            );
                        }
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
                                self.saved_decks.clone(),
                                self.collection.clone(),
                                deck_list,
                            ),
                        ));
                    }
                });
            });
    }

    fn render_opened_booster_pack(&mut self, ui: &mut Ui) {
        let Some(pack) = self.opened_booster_pack.clone() else {
            return;
        };

        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(format!("{} Booster Opened", pack.set_name))
                    .color(Color32::from_rgb(255, 200, 60))
                    .size(32.0)
                    .strong(),
            );
            ui.label(
                egui::RichText::new("All cards in this pack have been added to your collection.")
                    .color(Color32::from_rgb(180, 190, 215))
                    .size(15.0),
            );
            ui.add_space(10.0);

            let tray_width = ui.available_width().min(880.0);
            ui.allocate_ui_with_layout(
                vec2(tray_width, 710.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(14, 18, 30))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(50, 64, 96)))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            egui::Grid::new("opened_booster_pack_grid")
                                .num_columns(5)
                                .spacing(vec2(10.0, 12.0))
                                .show(ui, |ui| {
                                    for (index, card) in pack.cards.iter().enumerate() {
                                        Self::render_reward_card(ui, card, vec2(150.0, 205.0));
                                        if index % 5 == 4 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                },
            );
            ui.add_space(10.0);
            let continue_clicked = ui.button("Continue").clicked()
                || ui.ctx().input(|input| input.key_pressed(egui::Key::Enter));
            if continue_clicked {
                self.opened_booster_pack = None;
            }
        });
    }

    fn render_reward_card(ui: &mut Ui, booster_card: &BoosterCard, size: egui::Vec2) {
        let card_name = &booster_card.name;
        let card = Self::card_preview_data(card_name);
        let image_size = vec2(135.0, 178.0);
        let response = ui.allocate_ui_with_layout(
            size,
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                if let Some(texture) = TextureCache::get_card_texture_blocking(&card, ui.ctx()) {
                    let image = ui.add(
                        egui::Image::new(egui::ImageSource::Texture(
                            egui::load::SizedTexture::from_handle(&texture),
                        ))
                        .max_size(image_size),
                    );
                    if booster_card.is_foil {
                        Self::paint_foil_effect(ui, image.rect);
                    }
                } else {
                    ui.ctx().request_repaint();
                    ui.allocate_space(image_size);
                }
                ui.label(
                        egui::RichText::new(card_name)
                            .color(if booster_card.is_foil {
                                Color32::from_rgb(255, 222, 125)
                            } else {
                                Color32::from_rgb(230, 235, 250)
                            })
                        .size(11.0)
                        .strong(),
                );
            },
        );
        response.response.on_hover_ui(|ui| {
            ui.label(
                egui::RichText::new(if booster_card.is_foil {
                    format!("✦ Foil {card_name}")
                } else {
                    card_name.to_owned()
                })
                    .color(Color32::from_rgb(255, 200, 80))
                    .strong(),
            );
            if let Some(texture) = TextureCache::get_card_texture_blocking(&card, ui.ctx()) {
                let preview = ui.add(
                    egui::Image::new(egui::ImageSource::Texture(
                        egui::load::SizedTexture::from_handle(&texture),
                    ))
                    .max_size(vec2(310.0, 410.0)),
                );
                if booster_card.is_foil {
                    Self::paint_foil_effect(ui, preview.rect);
                }
            } else {
                ui.ctx().request_repaint();
                ui.allocate_space(vec2(310.0, 410.0));
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
            egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(205, 235, 245, 185),
            ),
            egui::StrokeKind::Inside,
        );
    }

    fn render_packs(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(18.0);
            ui.label(
                egui::RichText::new("Your Packs")
                    .color(Color32::from_rgb(255, 200, 60))
                    .size(30.0)
                    .strong(),
            );
            ui.label("Choose a pack to open it and add its cards to your collection.");
            ui.add_space(18.0);

            egui::ScrollArea::horizontal()
                .id_salt("owned_booster_packs")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                                for pack in self.unopened_booster_packs.clone() {
                                    let (rect, response) =
                                        ui.allocate_exact_size(vec2(175.0, 260.0), egui::Sense::click());
                                    let mut image_rect = rect;
                                    image_rect.max.y -= 26.0;
                                    if let Some(texture) = TextureCache::get_texture_blocking(
                                        "assets/images/beta_booster_1.webp",
                                        ui.ctx(),
                                    ) {
                                        egui::Image::new(egui::ImageSource::Texture(
                                            egui::load::SizedTexture::from_handle(&texture),
                                        ))
                                        .paint_at(ui, image_rect);
                                    } else {
                                        ui.ctx().request_repaint();
                                    }
                                    ui.painter().text(
                                        rect.center_bottom() - vec2(0.0, 12.0),
                                        egui::Align2::CENTER_CENTER,
                                        if response.hovered() { "Open pack" } else { "Beta Booster" },
                                        egui::FontId::proportional(13.0),
                                        if response.hovered() {
                                            Color32::from_rgb(255, 200, 80)
                                        } else {
                                            Color32::from_rgb(225, 230, 245)
                                        },
                                    );
                                    if response.clicked() {
                                        self.client
                                            .send(ClientMessage::OpenBoosterPack { pack_id: pack.id })
                                            .ok();
                                    }
                                    ui.add_space(8.0);
                                }
                    });
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
            power: card.get_unit_base().map(|unit| unit.power).unwrap_or_default(),
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
            } => {
                self.available_decks = available_decks.clone();
                self.saved_decks = saved_decks.clone();
                self.collection = collection.clone();
                self.player_id = Some(*player_id);
                self.player_name = username.clone();
                self.password.clear();
                self.auth_requested = false;
                self.auth_error = None;
                self.unopened_booster_packs = unopened_booster_packs.clone();
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
                self.unopened_booster_packs.retain(|unopened| unopened.id != *pack_id);
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
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(8, 8, 14)))
            .show_inside(ui, |ui| {
                let panel_h = ui.available_height();
                if self.opened_booster_pack.is_some() {
                    self.render_opened_booster_pack(ui);
                    return;
                }
                if self.show_packs {
                    self.render_packs(ui);
                    return;
                }
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
                    } else if self.selecting_starter_deck {
                        ui.label(
                            egui::RichText::new("Choose your starter deck")
                                .color(Color32::from_rgb(180, 180, 210))
                                .size(20.0),
                        );
                        ui.add_space(8.0);
                        ui.label("Its cards will be added to your collection.");
                        ui.add_space(16.0);
                        for deck in self.starter_decks.clone() {
                            if ui
                                .add_enabled(
                                    !self.auth_requested,
                                    egui::Button::new(deck.name()).min_size(vec2(260.0, 42.0)),
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
                        ui.label(
                            egui::RichText::new(if self.registering {
                                "Create account"
                            } else {
                                "Log in"
                            })
                            .color(Color32::from_rgb(180, 180, 210))
                            .size(20.0),
                        );
                        ui.add_space(12.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.username)
                                .hint_text("Username")
                                .desired_width(320.0),
                        );
                        ui.add_space(8.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.password)
                                .password(true)
                                .hint_text("Password")
                                .desired_width(320.0),
                        );
                        ui.add_space(10.0);

                        if let Some(error) = &self.auth_error {
                            ui.label(
                                egui::RichText::new(error)
                                    .color(Color32::from_rgb(220, 80, 60)),
                            );
                            ui.add_space(6.0);
                        }

                        let submit = ui.add_enabled(
                            !self.auth_requested && !self.username.is_empty() && !self.password.is_empty(),
                            egui::Button::new(if self.registering { "Create account" } else { "Log in" })
                                .min_size(vec2(180.0, 42.0)),
                        );
                        if submit.clicked() {
                            let message = if self.registering {
                                ClientMessage::Register {
                                    username: self.username.clone(),
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
                                self.auth_error = Some("Unable to reach the server".to_string());
                            }
                        }
                        if self.auth_requested {
                            ui.add_space(8.0);
                            ui.label("Authenticating...");
                        }
                        ui.add_space(10.0);
                        if ui
                            .link(if self.registering {
                                "Already have an account? Log in"
                            } else {
                                "Need an account? Register"
                            })
                            .clicked()
                        {
                            self.registering = !self.registering;
                            self.auth_error = None;
                        }
                    } else {
                        self.render_deck_selection(ui, &mut next_scene);
                    }
                });
            });

        next_scene
    }
}
