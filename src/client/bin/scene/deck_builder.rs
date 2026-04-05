use std::collections::HashMap;

use egui::epaint::Shape;
use egui::{Color32, Context, CornerRadius, Frame, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use sorcerers::deck::DeckList;
use sorcerers::{
    card::{ALL_CARDS, CardType, Rarity, Zone},
    game::{Element, Thresholds},
    networking::{self, message::PreconDeck},
};

use crate::{scene::Scene, texture_cache::TextureCache};

// ── Colors ──────────────────────────────────────────────────────────────────
const BG: Color32 = Color32::from_rgb(8, 8, 14);
const PANEL_BG: Color32 = Color32::from_rgba_premultiplied(15, 20, 38, 240);
const BORDER: Color32 = Color32::from_rgb(45, 60, 100);
const GOLD: Color32 = Color32::from_rgb(255, 200, 60);
const COL_FIRE: Color32 = Color32::from_rgb(220, 70, 40);
const COL_AIR: Color32 = Color32::from_rgb(160, 90, 220);
const COL_EARTH: Color32 = Color32::from_rgb(140, 100, 40);
const COL_WATER: Color32 = Color32::from_rgb(50, 150, 230);
const TEXT_DIM: Color32 = Color32::from_rgb(160, 165, 190);
const TEXT_BRIGHT: Color32 = Color32::from_rgb(220, 225, 255);

// ── Layout ───────────────────────────────────────────────────────────────────
const HEADER_H: f32 = 48.0;
const LEFT_FRAC: f32 = 0.62;
const CARD_THUMB_W: f32 = 44.0;
const CARD_THUMB_H: f32 = 62.0;
const ROW_H: f32 = 68.0;
const THRESH_SZ: f32 = 10.0;

// ── Element / type filter state ──────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum ElemFilter {
    All,
    Fire,
    Air,
    Earth,
    Water,
}

#[derive(Clone, PartialEq)]
enum TypeFilter {
    All,
    Minion,
    Site,
    Spell,
}

// ── Card metadata captured from ALL_CARDS ────────────────────────────────────
#[derive(Clone)]
pub struct CardEntry {
    pub name: String,
    pub card_type: CardType,
    pub zone: Zone, // Spellbook or Atlasbook
    #[allow(dead_code)]
    pub is_avatar: bool,
    pub rarity: Rarity,
    pub mana: u8,
    pub thresholds: Thresholds,
    pub image_path: String,
    pub power: Option<u16>,
    pub toughness: Option<u16>,
}

impl CardEntry {
    pub fn max_copies(&self) -> u8 {
        match self.rarity {
            Rarity::Ordinary => 4,
            Rarity::Exceptional => 3,
            Rarity::Elite => 2,
            Rarity::Unique => 1,
        }
    }

    #[allow(dead_code)]
    pub fn primary_element(&self) -> Option<Element> {
        let t = &self.thresholds;
        if t.fire > 0 {
            Some(Element::Fire)
        } else if t.air > 0 {
            Some(Element::Air)
        } else if t.earth > 0 {
            Some(Element::Earth)
        } else if t.water > 0 {
            Some(Element::Water)
        } else {
            None
        }
    }

    /// Fake CardData just for texture key + path.
    fn as_card_data(&self) -> sorcerers::card::CardData {
        use sorcerers::card::{CardData, Edition, Region, Zone};
        CardData {
            id: uuid::Uuid::nil(),
            name: self.name.clone(),
            owner_id: uuid::Uuid::nil(),
            controller_id: uuid::Uuid::nil(),
            tapped: false,
            edition: Edition::Beta,
            zone: Zone::Spellbook,
            region: Region::Surface,
            card_type: self.card_type.clone(),
            abilities: vec![],
            damage_taken: 0,
            bearer: None,
            rarity: self.rarity.clone(),
            power: self.power.unwrap_or(0),
            has_attachments: false,
            image_path: self.image_path.clone(),
            is_token: false,
        }
    }
}

// ── DeckBuilder scene ────────────────────────────────────────────────────────
pub struct DeckBuilder {
    client: networking::client::Client,
    player_id: Option<uuid::Uuid>,
    player_name: String,
    prev_available_decks: Vec<PreconDeck>,

    all_cards: Vec<CardEntry>,
    avatars: Vec<CardEntry>,

    // Deck contents: name → count
    deck_spells: HashMap<String, u8>,
    deck_sites: HashMap<String, u8>,
    selected_avatar: Option<String>,
    deck_name: String,

    // Filters
    search: String,
    elem_filter: ElemFilter,
    type_filter: TypeFilter,

    // Validation / save feedback
    save_error: Option<String>,
}

impl DeckBuilder {
    pub fn from_menu(
        client: networking::client::Client,
        player_id: Option<uuid::Uuid>,
        player_name: String,
        prev_available_decks: Vec<PreconDeck>,
    ) -> Self {
        let dummy_id = uuid::Uuid::nil();
        let mut all_cards: Vec<CardEntry> = Vec::new();
        let mut avatars: Vec<CardEntry> = Vec::new();

        for (_name, constructor) in ALL_CARDS {
            let card = constructor(dummy_id);
            if card.get_base().is_token {
                continue;
            }

            let base = card.get_base();
            let entry = CardEntry {
                name: card.get_name().to_string(),
                card_type: card.get_card_type(),
                zone: base.zone.clone(),
                is_avatar: card.is_avatar(),
                rarity: base.rarity.clone(),
                mana: base.cost.mana,
                thresholds: base.cost.thresholds.clone(),
                image_path: card.get_image_path(),
                power: card.get_unit_base().map(|u| u.power),
                toughness: card.get_unit_base().map(|u| u.toughness),
            };

            if card.is_avatar() {
                avatars.push(entry);
            } else {
                all_cards.push(entry);
            }
        }

        // Sort cards: sites first, then by name
        all_cards.sort_by(|a, b| {
            let a_site = matches!(a.zone, Zone::Atlasbook);
            let b_site = matches!(b.zone, Zone::Atlasbook);
            b_site.cmp(&a_site).then(a.name.cmp(&b.name))
        });
        avatars.sort_by(|a, b| a.name.cmp(&b.name));

        Self {
            client,
            player_id,
            player_name,
            prev_available_decks,
            all_cards,
            avatars,
            deck_spells: HashMap::new(),
            deck_sites: HashMap::new(),
            selected_avatar: None,
            deck_name: String::new(),
            search: String::new(),
            elem_filter: ElemFilter::All,
            type_filter: TypeFilter::All,
            save_error: None,
        }
    }

    pub fn render(&mut self, _ui: &mut Ui, ctx: &Context) -> Option<Scene> {
        let screen = ctx.screen_rect();
        let mut next_scene: Option<Scene> = None;

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG))
            .show(ctx, |ui| {
                // ── Header bar ────────────────────────────────────────────────
                let header_rect = Rect::from_min_size(screen.min, vec2(screen.width(), HEADER_H));
                ui.painter()
                    .rect_filled(header_rect, 0.0, Color32::from_rgb(12, 16, 28));
                ui.painter().line_segment(
                    [header_rect.left_bottom(), header_rect.right_bottom()],
                    Stroke::new(1.0, BORDER),
                );

                // Back button
                let back_rect = Rect::from_min_size(header_rect.min + vec2(12.0, 8.0), vec2(90.0, 32.0));
                let back_resp = ui.allocate_rect(back_rect, Sense::click());
                let back_col = if back_resp.hovered() {
                    Color32::from_rgb(80, 100, 160)
                } else {
                    Color32::from_rgb(40, 50, 90)
                };
                ui.painter().rect_filled(back_rect, CornerRadius::same(4), back_col);
                ui.painter().text(
                    back_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "← Back",
                    egui::FontId::proportional(15.0),
                    TEXT_BRIGHT,
                );
                if back_resp.clicked() {
                    next_scene = Some(self.back_to_menu());
                }

                // Title
                ui.painter().text(
                    header_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Deck Builder",
                    egui::FontId::proportional(24.0),
                    GOLD,
                );

                // Save button — active only when avatar, name, and deck sizes meet requirements
                let use_rect = Rect::from_min_size(header_rect.right_top() + vec2(-120.0, 8.0), vec2(108.0, 32.0));
                let spell_count_total: u32 = self.deck_spells.values().map(|&c| c as u32).sum();
                let site_count_total: u32 = self.deck_sites.values().map(|&c| c as u32).sum();
                let can_use = self.selected_avatar.is_some()
                    && !self.deck_name.trim().is_empty()
                    && spell_count_total >= 60
                    && site_count_total >= 30;
                let use_resp = ui.allocate_rect(use_rect, Sense::click());
                let use_bg = if !can_use {
                    Color32::from_rgb(30, 35, 55)
                } else if use_resp.hovered() {
                    Color32::from_rgb(30, 120, 60)
                } else {
                    Color32::from_rgb(20, 90, 45)
                };
                ui.painter().rect_filled(use_rect, CornerRadius::same(4), use_bg);
                let use_col = if can_use {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(80, 85, 100)
                };
                ui.painter().text(
                    use_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "💾 Save Deck",
                    egui::FontId::proportional(15.0),
                    use_col,
                );
                if use_resp.clicked() && can_use {
                    match self.try_save_deck() {
                        Ok(scene) => next_scene = Some(scene),
                        Err(e) => self.save_error = Some(e),
                    }
                }

                // Show save error or requirement hint below the header
                let hint = if let Some(ref err) = self.save_error.clone() {
                    Some((format!("⚠ {err}"), Color32::from_rgb(220, 80, 60)))
                } else if !can_use && (self.selected_avatar.is_some() || !self.deck_name.trim().is_empty()) {
                    // Show which requirements are missing
                    let mut missing = Vec::new();
                    if self.selected_avatar.is_none() { missing.push("avatar"); }
                    if self.deck_name.trim().is_empty() { missing.push("deck name"); }
                    if site_count_total < 30 {
                        // already shown in the panel, just summarise
                    }
                    if spell_count_total < 60 {
                        // already shown in the panel
                    }
                    let needs_sites = 30u32.saturating_sub(site_count_total);
                    let needs_spells = 60u32.saturating_sub(spell_count_total);
                    let mut parts: Vec<String> = missing.iter().map(|s| s.to_string()).collect();
                    if needs_sites > 0 { parts.push(format!("{needs_sites} more site(s)")); }
                    if needs_spells > 0 { parts.push(format!("{needs_spells} more spell(s)")); }
                    if parts.is_empty() { None } else {
                        Some((format!("Need: {}", parts.join(", ")), Color32::from_rgb(180, 160, 60)))
                    }
                } else {
                    None
                };
                if let Some((text, col)) = hint {
                    let err_pos = header_rect.center_bottom() + vec2(0.0, 2.0);
                    ui.painter().text(
                        err_pos,
                        egui::Align2::CENTER_TOP,
                        &text,
                        egui::FontId::proportional(12.0),
                        col,
                    );
                }

                // ── Main area below header ─────────────────────────────────────
                let body_top = screen.min.y + HEADER_H + 4.0;
                let body_rect = Rect::from_min_max(pos2(screen.min.x, body_top), screen.max);
                let left_w = body_rect.width() * LEFT_FRAC;
                let right_w = body_rect.width() - left_w;

                let left_rect = Rect::from_min_size(body_rect.min, vec2(left_w, body_rect.height()));
                let right_rect = Rect::from_min_size(
                    pos2(body_rect.min.x + left_w, body_rect.min.y),
                    vec2(right_w, body_rect.height()),
                );

                // Right panel background
                ui.painter().rect_filled(right_rect, 0.0, PANEL_BG);
                ui.painter().line_segment(
                    [right_rect.left_top(), right_rect.left_bottom()],
                    Stroke::new(1.0, BORDER),
                );

                // ── Left panel: card collection ────────────────────────────────
                let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
                self.render_left_panel(&mut left_ui, ctx, left_rect);

                // ── Right panel: deck summary ──────────────────────────────────
                let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
                self.render_right_panel(&mut right_ui, ctx, right_rect);
            });

        next_scene
    }

    fn render_left_panel(&mut self, ui: &mut Ui, ctx: &Context, rect: Rect) {
        let pad = 8.0;

        // Filter row
        let filter_h = 40.0;
        let filter_rect = Rect::from_min_size(rect.min + vec2(pad, pad), vec2(rect.width() - pad * 2.0, filter_h));

        let mut filter_ui = ui.new_child(egui::UiBuilder::new().max_rect(filter_rect));
        filter_ui.horizontal(|ui| {
            // Search field
            let te = egui::TextEdit::singleline(&mut self.search)
                .hint_text("🔍 Search…")
                .desired_width(160.0)
                .font(egui::FontId::proportional(14.0));
            ui.add(te);
            ui.add_space(8.0);

            // Element buttons
            for (label, filter, color) in [
                ("All", ElemFilter::All, TEXT_BRIGHT),
                ("🔥", ElemFilter::Fire, COL_FIRE),
                ("💨", ElemFilter::Air, COL_AIR),
                ("🌿", ElemFilter::Earth, COL_EARTH),
                ("💧", ElemFilter::Water, COL_WATER),
            ] {
                let active = self.elem_filter == filter;
                let bg = if active {
                    Color32::from_rgb(50, 70, 120)
                } else {
                    Color32::from_rgb(25, 30, 55)
                };
                let btn = egui::Button::new(egui::RichText::new(label).color(color).size(13.0))
                    .fill(bg)
                    .min_size(vec2(30.0, 26.0));
                if ui.add(btn).clicked() {
                    self.elem_filter = filter;
                }
            }
            ui.add_space(6.0);

            // Type buttons
            for (label, filter) in [
                ("All", TypeFilter::All),
                ("Minion", TypeFilter::Minion),
                ("Site", TypeFilter::Site),
                ("Spell", TypeFilter::Spell),
            ] {
                let active = self.type_filter == filter;
                let bg = if active {
                    Color32::from_rgb(50, 70, 120)
                } else {
                    Color32::from_rgb(25, 30, 55)
                };
                let btn = egui::Button::new(egui::RichText::new(label).color(TEXT_BRIGHT).size(13.0))
                    .fill(bg)
                    .min_size(vec2(46.0, 26.0));
                if ui.add(btn).clicked() {
                    self.type_filter = filter;
                }
            }
        });

        // Card list
        let list_top = rect.min.y + pad + filter_h + 4.0;
        let list_rect = Rect::from_min_max(
            pos2(rect.min.x + pad, list_top),
            pos2(rect.max.x - pad, rect.max.y - pad),
        );

        let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));

        let search_lower = self.search.to_lowercase();
        let elem_filter = self.elem_filter.clone();
        let type_filter = self.type_filter.clone();

        // Collect filtered cards
        let filtered: Vec<CardEntry> = self
            .all_cards
            .iter()
            .filter(|c| {
                if !search_lower.is_empty() && !c.name.to_lowercase().contains(&search_lower) {
                    return false;
                }
                match &elem_filter {
                    ElemFilter::All => {}
                    ElemFilter::Fire => {
                        if c.thresholds.fire == 0 {
                            return false;
                        }
                    }
                    ElemFilter::Air => {
                        if c.thresholds.air == 0 {
                            return false;
                        }
                    }
                    ElemFilter::Earth => {
                        if c.thresholds.earth == 0 {
                            return false;
                        }
                    }
                    ElemFilter::Water => {
                        if c.thresholds.water == 0 {
                            return false;
                        }
                    }
                }
                match &type_filter {
                    TypeFilter::All => {}
                    TypeFilter::Minion => {
                        if c.card_type != CardType::Minion {
                            return false;
                        }
                    }
                    TypeFilter::Site => {
                        if !matches!(c.zone, Zone::Atlasbook) {
                            return false;
                        }
                    }
                    TypeFilter::Spell => {
                        if matches!(c.zone, Zone::Atlasbook) || c.card_type == CardType::Minion {
                            return false;
                        }
                    }
                }
                true
            })
            .cloned()
            .collect();

        ScrollArea::vertical().id_salt("card_list").show(&mut list_ui, |ui| {
            for entry in &filtered {
                let is_site = matches!(entry.zone, Zone::Atlasbook);
                let map = if is_site { &self.deck_sites } else { &self.deck_spells };
                let current_count = map.get(&entry.name).copied().unwrap_or(0);
                let max_copies = entry.max_copies();

                let row_resp = ui.allocate_response(vec2(ui.available_width(), ROW_H), Sense::hover());
                let row_rect = row_resp.rect;

                // Row background (alternating)
                let row_bg = Color32::from_rgba_premultiplied(20, 25, 45, 200);
                ui.painter().rect_filled(row_rect, CornerRadius::same(3), row_bg);

                // Thumbnail
                let thumb_rect = Rect::from_min_size(
                    row_rect.min + vec2(4.0, (ROW_H - CARD_THUMB_H) / 2.0),
                    vec2(CARD_THUMB_W, CARD_THUMB_H),
                );

                let fake_card_data = entry.as_card_data();
                if let Some(tex) = TextureCache::get_card_texture_blocking(&fake_card_data, ctx) {
                    egui::Image::new(egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&tex)))
                        .max_size(vec2(CARD_THUMB_W, CARD_THUMB_H))
                        .paint_at(ui, thumb_rect);
                } else {
                    ui.painter()
                        .rect_filled(thumb_rect, CornerRadius::same(2), Color32::from_rgb(30, 40, 60));
                    ui.painter().text(
                        thumb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "?",
                        egui::FontId::proportional(18.0),
                        TEXT_DIM,
                    );
                }

                // Card info
                let info_x = thumb_rect.max.x + 8.0;
                let name_pos = pos2(info_x, row_rect.min.y + 10.0);
                ui.painter().text(
                    name_pos,
                    egui::Align2::LEFT_TOP,
                    &entry.name,
                    egui::FontId::proportional(14.0),
                    TEXT_BRIGHT,
                );

                // Type + rarity
                let type_label = match entry.card_type {
                    CardType::Minion => "Minion",
                    CardType::Site => "Site",
                    CardType::Magic => "Magic",
                    CardType::Artifact => "Artifact",
                    CardType::Aura => "Aura",
                    CardType::Avatar => "Avatar",
                };
                let rarity_label = match entry.rarity {
                    Rarity::Ordinary => "●",
                    Rarity::Exceptional => "◆",
                    Rarity::Elite => "★",
                    Rarity::Unique => "♦",
                };
                let rarity_color = match entry.rarity {
                    Rarity::Ordinary => Color32::from_rgb(160, 160, 160),
                    Rarity::Exceptional => Color32::from_rgb(80, 180, 255),
                    Rarity::Elite => Color32::from_rgb(200, 160, 60),
                    Rarity::Unique => Color32::from_rgb(200, 80, 200),
                };
                let sub_pos = pos2(info_x, row_rect.min.y + 28.0);
                ui.painter().text(
                    sub_pos,
                    egui::Align2::LEFT_TOP,
                    format!("{type_label}  {rarity_label}"),
                    egui::FontId::proportional(12.0),
                    TEXT_DIM,
                );
                // rarity glyph in color
                let rarity_x = sub_pos.x
                    + ui.painter()
                        .layout_no_wrap(format!("{type_label}  "), egui::FontId::proportional(12.0), TEXT_DIM)
                        .rect
                        .width();
                ui.painter().text(
                    pos2(rarity_x, sub_pos.y),
                    egui::Align2::LEFT_TOP,
                    rarity_label,
                    egui::FontId::proportional(12.0),
                    rarity_color,
                );

                // Mana + thresholds
                let cost_y = row_rect.min.y + 44.0;
                let mut cx = info_x;
                if entry.mana > 0 {
                    ui.painter().text(
                        pos2(cx, cost_y),
                        egui::Align2::LEFT_TOP,
                        format!("{}", entry.mana),
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(180, 210, 255),
                    );
                    cx += 18.0;
                }
                cx = draw_thresh_symbols(ui.painter(), cx, cost_y + 1.0, &entry.thresholds);

                // Power/toughness for minions
                if let (Some(pow), Some(tough)) = (entry.power, entry.toughness) {
                    ui.painter().text(
                        pos2(cx + 8.0, cost_y),
                        egui::Align2::LEFT_TOP,
                        format!("{pow}/{tough}"),
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(200, 200, 120),
                    );
                }

                // +/- buttons on the right
                let btn_w = 26.0;
                let btn_h = 26.0;
                let minus_rect = Rect::from_min_size(
                    pos2(row_rect.max.x - btn_w * 2.0 - 36.0, row_rect.center().y - btn_h / 2.0),
                    vec2(btn_w, btn_h),
                );
                let count_rect = Rect::from_min_size(
                    pos2(row_rect.max.x - btn_w - 32.0, row_rect.center().y - btn_h / 2.0),
                    vec2(28.0, btn_h),
                );
                let plus_rect = Rect::from_min_size(
                    pos2(row_rect.max.x - btn_w - 4.0, row_rect.center().y - btn_h / 2.0),
                    vec2(btn_w, btn_h),
                );

                // Minus
                let minus_resp = ui.allocate_rect(minus_rect, Sense::click());
                let minus_bg = if current_count > 0 && minus_resp.hovered() {
                    Color32::from_rgb(130, 40, 40)
                } else {
                    Color32::from_rgb(55, 25, 25)
                };
                ui.painter().rect_filled(minus_rect, CornerRadius::same(3), minus_bg);
                ui.painter().text(
                    minus_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "−",
                    egui::FontId::proportional(16.0),
                    Color32::WHITE,
                );
                if minus_resp.clicked() && current_count > 0 {
                    let map = if is_site {
                        &mut self.deck_sites
                    } else {
                        &mut self.deck_spells
                    };
                    let count = map.entry(entry.name.clone()).or_insert(0);
                    if *count > 0 {
                        *count -= 1;
                    }
                    if *count == 0 {
                        map.remove(&entry.name);
                    }
                }

                // Count
                ui.painter().text(
                    count_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{current_count}"),
                    egui::FontId::proportional(15.0),
                    TEXT_BRIGHT,
                );

                // Plus
                let plus_resp = ui.allocate_rect(plus_rect, Sense::click());
                let can_add = current_count < max_copies;
                let plus_bg = if can_add && plus_resp.hovered() {
                    Color32::from_rgb(30, 110, 50)
                } else if can_add {
                    Color32::from_rgb(20, 70, 35)
                } else {
                    Color32::from_rgb(25, 30, 25)
                };
                ui.painter().rect_filled(plus_rect, CornerRadius::same(3), plus_bg);
                let plus_col = if can_add {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(70, 75, 70)
                };
                ui.painter().text(
                    plus_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "+",
                    egui::FontId::proportional(16.0),
                    plus_col,
                );
                if plus_resp.clicked() && can_add {
                    let map = if is_site {
                        &mut self.deck_sites
                    } else {
                        &mut self.deck_spells
                    };
                    let count = map.entry(entry.name.clone()).or_insert(0);
                    *count += 1;
                }

                // Separator
                ui.painter().line_segment(
                    [row_rect.left_bottom(), row_rect.right_bottom()],
                    Stroke::new(0.5, Color32::from_rgb(30, 35, 60)),
                );

                ui.add_space(2.0);
            }
        });
    }

    fn render_right_panel(&mut self, ui: &mut Ui, ctx: &Context, rect: Rect) {
        let pad = 10.0;
        let inner = rect.shrink(pad);

        // Deck name input
        let mut y = inner.min.y;
        ui.painter().text(
            pos2(inner.min.x, y),
            egui::Align2::LEFT_TOP,
            "Deck Name",
            egui::FontId::proportional(13.0),
            TEXT_DIM,
        );
        y += 18.0;
        let name_rect = Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), 26.0));
        let name_border_col = if self.deck_name.trim().is_empty() {
            Color32::from_rgb(120, 50, 50)
        } else {
            Color32::from_rgb(60, 90, 140)
        };
        ui.painter().rect_filled(name_rect, CornerRadius::same(3), Color32::from_rgb(18, 22, 40));
        ui.painter().rect_stroke(name_rect, CornerRadius::same(3), egui::Stroke::new(1.0, name_border_col), StrokeKind::Outside);
        let te = egui::TextEdit::singleline(&mut self.deck_name)
            .hint_text("Enter deck name…")
            .font(egui::FontId::proportional(13.0))
            .text_color(TEXT_BRIGHT)
            .background_color(Color32::TRANSPARENT)
            .frame(false);
        ui.put(name_rect.shrink(4.0), te);
        y += 34.0;

        // Avatar section header
        ui.painter().text(
            pos2(inner.min.x, y),
            egui::Align2::LEFT_TOP,
            "Avatar",
            egui::FontId::proportional(16.0),
            GOLD,
        );
        y += 22.0;

        // Avatar portraits
        let avatar_sz = vec2(54.0, 76.0);
        let avatars_per_row = ((inner.width() + 8.0) / (avatar_sz.x + 8.0)).floor() as usize;
        let avatars_per_row = avatars_per_row.max(1);

        let avatars: Vec<CardEntry> = self.avatars.clone();
        for (i, av) in avatars.iter().enumerate() {
            let col = i % avatars_per_row;
            let row_i = i / avatars_per_row;
            let av_rect = Rect::from_min_size(
                pos2(
                    inner.min.x + col as f32 * (avatar_sz.x + 8.0),
                    y + row_i as f32 * (avatar_sz.y + 28.0),
                ),
                avatar_sz,
            );

            let is_selected = self.selected_avatar.as_deref() == Some(&av.name);
            let av_resp = ui.allocate_rect(av_rect, Sense::click());

            if is_selected {
                ui.painter().rect_stroke(
                    av_rect.expand(2.0),
                    CornerRadius::same(4),
                    Stroke::new(2.0, GOLD),
                    StrokeKind::Outside,
                );
            }

            let fake = av.as_card_data();
            if let Some(tex) = TextureCache::get_card_texture_blocking(&fake, ctx) {
                egui::Image::new(egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&tex)))
                    .max_size(avatar_sz)
                    .paint_at(ui, av_rect);
            } else {
                ui.painter()
                    .rect_filled(av_rect, CornerRadius::same(3), Color32::from_rgb(30, 40, 60));
                ui.painter().text(
                    av_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "?",
                    egui::FontId::proportional(18.0),
                    TEXT_DIM,
                );
            }

            // Name below portrait
            let name_pos = pos2(av_rect.center().x, av_rect.max.y + 3.0);
            let av_name_col = if is_selected { GOLD } else { TEXT_DIM };
            ui.painter().text(
                name_pos,
                egui::Align2::CENTER_TOP,
                &av.name,
                egui::FontId::proportional(10.0),
                av_name_col,
            );

            if av_resp.clicked() {
                self.selected_avatar = Some(av.name.clone());
            }
        }

        let rows_count = (avatars.len() + avatars_per_row - 1) / avatars_per_row;
        y += rows_count as f32 * (avatar_sz.y + 28.0) + 12.0;

        // Separator
        ui.painter()
            .line_segment([pos2(inner.min.x, y), pos2(inner.max.x, y)], Stroke::new(1.0, BORDER));
        y += 8.0;

        // Deck list area
        let deck_list_rect = Rect::from_min_max(pos2(inner.min.x, y), pos2(inner.max.x, rect.max.y - pad - 30.0));
        let mut deck_ui = ui.new_child(egui::UiBuilder::new().max_rect(deck_list_rect));

        let site_count: u32 = self.deck_sites.values().map(|&c| c as u32).sum();
        let spell_count: u32 = self.deck_spells.values().map(|&c| c as u32).sum();

        ScrollArea::vertical().id_salt("deck_list").show(&mut deck_ui, |ui| {
            // Atlas section
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Atlas")
                        .color(Color32::from_rgb(100, 200, 100))
                        .size(14.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("({site_count})"))
                        .color(TEXT_DIM)
                        .size(13.0),
                );
            });
            ui.add_space(4.0);

            let mut sites_to_remove: Option<String> = None;
            let mut sites_snapshot: Vec<(String, u8)> = self.deck_sites.iter().map(|(k, &v)| (k.clone(), v)).collect();
            sites_snapshot.sort_by(|a, b| a.0.cmp(&b.0));

            for (name, count) in &sites_snapshot {
                ui.horizontal(|ui| {
                    if ui.small_button("×").clicked() {
                        sites_to_remove = Some(name.clone());
                    }
                    ui.label(
                        egui::RichText::new(format!("{count}×  {name}"))
                            .color(TEXT_BRIGHT)
                            .size(13.0),
                    );
                });
            }
            if let Some(rm) = sites_to_remove {
                self.deck_sites.remove(&rm);
            }

            ui.add_space(10.0);

            // Spellbook section
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Spellbook")
                        .color(Color32::from_rgb(120, 160, 255))
                        .size(14.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("({spell_count})"))
                        .color(TEXT_DIM)
                        .size(13.0),
                );
            });
            ui.add_space(4.0);

            let mut spells_to_remove: Option<String> = None;
            let mut spells_snapshot: Vec<(String, u8)> =
                self.deck_spells.iter().map(|(k, &v)| (k.clone(), v)).collect();
            spells_snapshot.sort_by(|a, b| a.0.cmp(&b.0));

            for (name, count) in &spells_snapshot {
                ui.horizontal(|ui| {
                    if ui.small_button("×").clicked() {
                        spells_to_remove = Some(name.clone());
                    }
                    ui.label(
                        egui::RichText::new(format!("{count}×  {name}"))
                            .color(TEXT_BRIGHT)
                            .size(13.0),
                    );
                });
            }
            if let Some(rm) = spells_to_remove {
                self.deck_spells.remove(&rm);
            }
        });

        // Total counts at the bottom — color-coded to show progress toward required minimums
        let totals_y = rect.max.y - pad - 40.0;

        let sites_ok = site_count >= 30;
        let spells_ok = spell_count >= 60;
        let site_col = if sites_ok { Color32::from_rgb(100, 210, 120) } else { Color32::from_rgb(220, 120, 60) };
        let spell_col = if spells_ok { Color32::from_rgb(100, 210, 120) } else { Color32::from_rgb(220, 120, 60) };

        let atlas_text = format!("Atlas: {site_count}/30");
        let spell_text = format!("Spellbook: {spell_count}/60");

        ui.painter().text(
            pos2(inner.min.x, totals_y),
            egui::Align2::LEFT_TOP,
            &atlas_text,
            egui::FontId::proportional(12.0),
            site_col,
        );
        ui.painter().text(
            pos2(inner.min.x, totals_y + 16.0),
            egui::Align2::LEFT_TOP,
            &spell_text,
            egui::FontId::proportional(12.0),
            spell_col,
        );
    }

    fn back_to_menu(&self) -> Scene {
        Scene::Menu(crate::scene::menu::Menu::restore(
            self.client.clone(),
            self.player_id,
            self.player_name.clone(),
            self.prev_available_decks.clone(),
        ))
    }

    fn try_save_deck(&mut self) -> Result<Scene, String> {
        let avatar = self.selected_avatar.clone().unwrap_or_default();
        let name = self.deck_name.trim().to_string();

        // Flatten deck_spells/sites into lists (with repetition)
        let mut spells: Vec<String> = Vec::new();
        for (card_name, &count) in &self.deck_spells {
            for _ in 0..count {
                spells.push(card_name.clone());
            }
        }
        let mut sites: Vec<String> = Vec::new();
        for (card_name, &count) in &self.deck_sites {
            for _ in 0..count {
                sites.push(card_name.clone());
            }
        }

        let deck_list = DeckList { name, avatar, spells, sites };

        // Validate before saving
        deck_list.validate()?;

        // Save to disk
        deck_list.save().map_err(|e| format!("Failed to save: {e}"))?;

        Ok(Scene::Menu(crate::scene::menu::Menu::restore(
            self.client.clone(),
            self.player_id,
            self.player_name.clone(),
            self.prev_available_decks.clone(),
        )))
    }

    pub fn process_input(&mut self, _ctx: &Context) -> Option<Scene> {
        None
    }
}

/// Draw threshold symbols at (x, y), return new x offset.
fn draw_thresh_symbols(painter: &egui::Painter, mut x: f32, y: f32, t: &Thresholds) -> f32 {
    let s = THRESH_SZ;
    for (count, element) in [
        (t.fire, Element::Fire),
        (t.air, Element::Air),
        (t.earth, Element::Earth),
        (t.water, Element::Water),
    ] {
        if count == 0 {
            continue;
        }
        let col = elem_color(element.clone());
        let is_upward = matches!(element, Element::Fire | Element::Air);
        let has_midline = matches!(element, Element::Air | Element::Earth);

        let (v1, v2, v3) = if is_upward {
            (pos2(x + s / 2.0, y), pos2(x, y + s), pos2(x + s, y + s))
        } else {
            (pos2(x, y), pos2(x + s, y), pos2(x + s / 2.0, y + s))
        };
        painter.add(Shape::closed_line(vec![v1, v2, v3], Stroke::new(1.2, col)));
        if has_midline {
            let mid_y = y + s / 2.0;
            painter.line_segment([pos2(x, mid_y), pos2(x + s, mid_y)], Stroke::new(1.2, col));
        }
        if count > 1 {
            painter.text(
                pos2(x + s + 2.0, y),
                egui::Align2::LEFT_TOP,
                count.to_string(),
                egui::FontId::proportional(10.0),
                col,
            );
            x += 14.0;
        }
        x += s + 4.0;
    }
    x
}

fn elem_color(el: Element) -> Color32 {
    match el {
        Element::Fire => COL_FIRE,
        Element::Air => COL_AIR,
        Element::Earth => COL_EARTH,
        Element::Water => COL_WATER,
    }
}
