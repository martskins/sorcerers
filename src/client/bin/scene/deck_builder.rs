use crate::{element_icon, render, scene::Scene, texture_cache::TextureCache, theme};
use egui::{
    Color32, Context, CornerRadius, Frame, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};
use sorcerers::collection::CollectedCard;
use sorcerers::deck::precon::PreconDeck;
use sorcerers::game::PlayerId;
use sorcerers::{
    card::{ALL_CARDS, CardType, Edition, Rarity},
    game::Element,
    networking,
    zone::Zone,
};
use std::collections::HashMap;
use unidecode::unidecode;

mod save;
mod types;

use types::{CardEntry, ElemFilter, OwnershipFilter, SetFilter, TypeFilter};

// ── Colors ──────────────────────────────────────────────────────────────────
const BG: Color32 = theme::APP_BG;
const PANEL_BG: Color32 = theme::PANEL_BG;
const BORDER: Color32 = theme::PANEL_BORDER;
const GOLD: Color32 = Color32::from_rgb(255, 200, 60);
const TEXT_DIM: Color32 = Color32::from_rgb(160, 165, 190);
const TEXT_BRIGHT: Color32 = Color32::from_rgb(220, 225, 255);
const SUCCESS: Color32 = theme::TURN_READY;
const ERROR: Color32 = Color32::from_rgb(220, 80, 60);

// ── Layout ───────────────────────────────────────────────────────────────────
const HEADER_H: f32 = 48.0;
const LEFT_FRAC: f32 = 0.62;
const CARD_THUMB_W: f32 = 44.0;
const CARD_THUMB_H: f32 = 62.0;
const ROW_H: f32 = 92.0;
const THRESH_SZ: f32 = 10.0;

// ── DeckBuilder scene ────────────────────────────────────────────────────────
pub struct DeckBuilder {
    client: networking::client::Client,
    player_id: Option<PlayerId>,
    player_name: String,
    prev_available_decks: Vec<PreconDeck>,
    prev_saved_decks: Vec<sorcerers::deck::DeckList>,
    collection_entries: Vec<CollectedCard>,
    collection: HashMap<String, u8>,

    all_cards: Vec<CardEntry>,
    avatars: Vec<CardEntry>,

    // Deck contents: (name, is_foil) -> count
    deck_spells: HashMap<(String, bool), u8>,
    deck_sites: HashMap<(String, bool), u8>,
    selected_avatar: Option<String>,
    deck_name: String,

    // Filters
    search: String,
    set_filter: SetFilter,
    ownership_filter: OwnershipFilter,
    elem_filter: ElemFilter,
    type_filter: TypeFilter,

    // Validation / save feedback
    save_error: Option<String>,

    // Card preview: (entry, row_rect center-right position)
    hovered_card: Option<(CardEntry, egui::Pos2)>,
}

impl DeckBuilder {
    fn collection_counts(&self, name: &str) -> (u8, u8) {
        self.collection_entries.iter().fold((0, 0), |(regular, foil), card| {
            if card.name != name {
                (regular, foil)
            } else if card.is_foil {
                (regular, foil.saturating_add(card.count))
            } else {
                (regular.saturating_add(card.count), foil)
            }
        })
    }

    pub fn from_menu(
        client: networking::client::Client,
        player_id: Option<PlayerId>,
        player_name: String,
        prev_available_decks: Vec<PreconDeck>,
        prev_saved_decks: Vec<sorcerers::deck::DeckList>,
        collection: Vec<CollectedCard>,
    ) -> Self {
        Self::build(client, player_id, player_name, prev_available_decks, prev_saved_decks, collection, None)
    }

    /// Open the deck builder pre-populated with an existing saved deck for editing.
    pub fn from_deck_list(
        client: networking::client::Client,
        player_id: Option<PlayerId>,
        player_name: String,
        prev_available_decks: Vec<PreconDeck>,
        prev_saved_decks: Vec<sorcerers::deck::DeckList>,
        collection: Vec<CollectedCard>,
        deck: sorcerers::deck::DeckList,
    ) -> Self {
        Self::build(
            client,
            player_id,
            player_name,
            prev_available_decks,
            prev_saved_decks,
            collection,
            Some(deck),
        )
    }

    fn build(
        client: networking::client::Client,
        player_id: Option<PlayerId>,
        player_name: String,
        prev_available_decks: Vec<PreconDeck>,
        prev_saved_decks: Vec<sorcerers::deck::DeckList>,
        collection: Vec<CollectedCard>,
        existing: Option<sorcerers::deck::DeckList>,
    ) -> Self {
        let collection_entries = collection;
        let collection: HashMap<String, u8> = collection_entries.iter().fold(HashMap::new(), |mut cards, card| {
            let count = cards.entry(card.name.clone()).or_insert(0u8);
            *count = count.saturating_add(card.count);
            cards
        });
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
                edition: base.edition.clone(),
                card_type: card.get_card_type(),
                zone: base.zone.clone(),
                rarity: base.rarity.clone(),
                mana: base.costs.printed_mana_value().unwrap_or_default(),
                thresholds: base.costs.printed_thresholds().clone(),
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

        // Pre-populate from existing deck if editing
        let (deck_spells, deck_sites, selected_avatar, deck_name) = if let Some(ref dl) = existing {
            let spells: HashMap<(String, bool), u8> = dl
                .spells
                .iter()
                .map(|c| ((c.name.clone(), c.is_foil), c.count))
                .collect();
            let sites: HashMap<(String, bool), u8> = dl
                .sites
                .iter()
                .map(|c| ((c.name.clone(), c.is_foil), c.count))
                .collect();
            (spells, sites, Some(dl.avatar.clone()), dl.name.clone())
        } else {
            (HashMap::new(), HashMap::new(), None, String::new())
        };

        Self {
            client,
            player_id,
            player_name,
            prev_available_decks,
            prev_saved_decks,
            collection_entries,
            collection,
            all_cards,
            avatars,
            deck_spells,
            deck_sites,
            selected_avatar,
            deck_name,
            search: String::new(),
            set_filter: SetFilter::All,
            ownership_filter: OwnershipFilter::All,
            elem_filter: ElemFilter::All,
            type_filter: TypeFilter::All,
            save_error: None,
            hovered_card: None,
        }
    }

    pub fn render(&mut self, ui: &mut Ui) -> Option<Scene> {
        let screen = ui.ctx().content_rect();
        let mut next_scene: Option<Scene> = None;

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG))
            .show_inside(ui, |ui| {
                // ── Header bar ────────────────────────────────────────────────
                let header_rect = Rect::from_min_size(screen.min, vec2(screen.width(), HEADER_H));
                ui.painter()
                    .rect_filled(header_rect, 0.0, Color32::from_rgb(12, 16, 28));
                ui.painter().line_segment(
                    [header_rect.left_bottom(), header_rect.right_bottom()],
                    Stroke::new(1.0, BORDER),
                );

                // Back button
                let back_rect =
                    Rect::from_min_size(header_rect.min + vec2(12.0, 8.0), vec2(90.0, 32.0));
                let back_resp = ui.allocate_rect(back_rect, Sense::click());
                let back_col = if back_resp.hovered() {
                    Color32::from_rgb(80, 100, 160)
                } else {
                    Color32::from_rgb(40, 50, 90)
                };
                ui.painter()
                    .rect_filled(back_rect, CornerRadius::same(4), back_col);
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

                ui.painter().text(
                    pos2(header_rect.center().x, header_rect.center().y - 7.0),
                    egui::Align2::CENTER_CENTER,
                    "Deck Builder",
                    egui::FontId::proportional(22.0),
                    GOLD,
                );
                ui.painter().text(
                    pos2(header_rect.center().x, header_rect.center().y + 11.0),
                    egui::Align2::CENTER_CENTER,
                    "Build from your collection",
                    egui::FontId::proportional(12.0),
                    TEXT_DIM,
                );

                // Save button — active only when avatar, name, and deck sizes meet requirements
                let use_rect = Rect::from_min_size(
                    header_rect.right_top() + vec2(-120.0, 8.0),
                    vec2(108.0, 32.0),
                );
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
                    theme::ACTION_HOVERED
                } else {
                    theme::ACTION
                };
                ui.painter()
                    .rect_filled(use_rect, CornerRadius::same(4), use_bg);
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
                    Some((format!("⚠ {err}"), ERROR))
                } else if !can_use
                    && (self.selected_avatar.is_some() || !self.deck_name.trim().is_empty())
                {
                    // Show which requirements are missing
                    let mut missing = Vec::new();
                    if self.selected_avatar.is_none() {
                        missing.push("avatar");
                    }
                    if self.deck_name.trim().is_empty() {
                        missing.push("deck name");
                    }
                    if site_count_total < 30 {
                        // already shown in the panel, just summarise
                    }
                    if spell_count_total < 60 {
                        // already shown in the panel
                    }
                    let needs_sites = 30u32.saturating_sub(site_count_total);
                    let needs_spells = 60u32.saturating_sub(spell_count_total);
                    let mut parts: Vec<String> = missing.iter().map(|s| s.to_string()).collect();
                    if needs_sites > 0 {
                        parts.push(format!("{needs_sites} more site(s)"));
                    }
                    if needs_spells > 0 {
                        parts.push(format!("{needs_spells} more spell(s)"));
                    }
                    if parts.is_empty() {
                        None
                    } else {
                        Some((
                            format!("Deck checklist: {}", parts.join(", ")),
                            Color32::from_rgb(180, 160, 60),
                        ))
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

                let left_rect =
                    Rect::from_min_size(body_rect.min, vec2(left_w, body_rect.height()));
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
                self.hovered_card = None; // reset each frame; render_left_panel sets it
                let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
                self.render_left_panel(&mut left_ui, ui.ctx(), left_rect);

                // ── Right panel: deck summary ──────────────────────────────────
                let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
                self.render_right_panel(&mut right_ui, ui.ctx(), right_rect);

                // ── Card preview popup (floating, over everything) ─────────────
                if render::card_preview_requested(ui.ctx())
                    && let Some((ref entry, anchor)) = self.hovered_card.clone()
                {
                    Self::draw_card_preview(ui.ctx(), entry, anchor, screen);
                }
            });

        next_scene
    }

    fn render_left_panel(&mut self, ui: &mut Ui, ctx: &Context, rect: Rect) {
        let pad = 8.0;

        // Filter row
        // Dropdowns are taller than the icon buttons; reserve a little clearance before cards.
        let filter_h = 64.0;
        let filter_rect = Rect::from_min_size(
            rect.min + vec2(pad, pad),
            vec2(rect.width() - pad * 2.0, filter_h),
        );

        let search_lower = unidecode(&self.search).to_lowercase();
        let set_filter = self.set_filter.clone();
        let ownership_filter = self.ownership_filter.clone();
        let elem_filter = self.elem_filter.clone();
        let type_filter = self.type_filter.clone();

        // Collect filtered cards
        let filtered: Vec<CardEntry> = self
            .all_cards
            .iter()
            .filter(|c| {
                if !search_lower.is_empty()
                    && !unidecode(&c.name).to_lowercase().contains(&search_lower)
                {
                    return false;
                }
                if let SetFilter::Edition(edition) = &set_filter
                    && &c.edition != edition
                {
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

        let mut filter_ui = ui.new_child(egui::UiBuilder::new().max_rect(filter_rect));
        filter_ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            // Search field
            let te = egui::TextEdit::singleline(&mut self.search)
                .hint_text("🔍 Search…")
                .desired_width(160.0)
                .background_color(crate::theme::SURFACE_INSET)
                .font(egui::FontId::proportional(14.0));
            ui.add(te);
            ui.add_space(8.0);

            ui.scope(|ui| {
                ui.spacing_mut().interact_size.y = 26.0;
                egui::ComboBox::from_id_salt("deck_builder_set_filter")
                    .selected_text(self.set_filter.label())
                    .width(104.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.set_filter,
                            SetFilter::All,
                            SetFilter::All.label(),
                        );
                        for edition in [
                            Edition::Beta,
                            Edition::Alpha,
                            Edition::ArthurianLegends,
                            Edition::Dragonlord,
                            Edition::Gothic,
                        ] {
                            let filter = SetFilter::Edition(edition);
                            ui.selectable_value(
                                &mut self.set_filter,
                                filter.clone(),
                                filter.label(),
                            );
                        }
                    });
            });
            ui.add_space(6.0);

            ui.scope(|ui| {
                ui.spacing_mut().interact_size.y = 26.0;
                egui::ComboBox::from_id_salt("deck_builder_ownership_filter")
                    .selected_text(self.ownership_filter.label())
                    .width(92.0)
                    .show_ui(ui, |ui| {
                        for filter in [
                            OwnershipFilter::All,
                            OwnershipFilter::Owned,
                            OwnershipFilter::Unowned,
                        ] {
                            ui.selectable_value(
                                &mut self.ownership_filter,
                                filter.clone(),
                                filter.label(),
                            );
                        }
                    });
            });
            ui.add_space(6.0);

            // Element buttons — "All" as text, elements as triangle icons
            let btn_sz = vec2(30.0, 26.0);

            // "All" text button
            {
                let active = self.elem_filter == ElemFilter::All;
                let bg = if active {
                    Color32::from_rgb(50, 70, 120)
                } else {
                    Color32::from_rgb(25, 30, 55)
                };
                let btn =
                    egui::Button::new(egui::RichText::new("All").color(TEXT_BRIGHT).size(13.0))
                        .fill(bg)
                        .min_size(btn_sz);
                if ui.add(btn).clicked() {
                    self.elem_filter = ElemFilter::All;
                }
            }

            // Icon buttons for each element
            for (filter, element) in [
                (ElemFilter::Fire, Element::Fire),
                (ElemFilter::Air, Element::Air),
                (ElemFilter::Earth, Element::Earth),
                (ElemFilter::Water, Element::Water),
            ] {
                let active = self.elem_filter == filter;
                if element_icon::element_filter_button(ui, &element, active, btn_sz, 14.0, 1.5) {
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
                let btn =
                    egui::Button::new(egui::RichText::new(label).color(TEXT_BRIGHT).size(13.0))
                        .fill(bg)
                        .min_size(vec2(46.0, 26.0));
                if ui.add(btn).clicked() {
                    self.type_filter = filter;
                }
            }

            let filtered_count = filtered.len();
            let label = egui::Label::new(format!("{} card(s)", filtered_count));
            ui.add_space(8.0);
            ui.add(label);
        });

        // Card list
        let list_top = rect.min.y + pad + filter_h + 4.0;
        let list_rect = Rect::from_min_max(
            pos2(rect.min.x + pad, list_top),
            pos2(rect.max.x - pad, rect.max.y - pad),
        );

        let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));

        if filtered.is_empty() {
            let empty_rect = list_rect.shrink2(vec2(16.0, 24.0));
            list_ui.painter().rect_filled(
                empty_rect,
                CornerRadius::same(6),
                Color32::from_rgba_premultiplied(18, 23, 35, 210),
            );
            list_ui.painter().rect_stroke(
                empty_rect,
                CornerRadius::same(6),
                Stroke::new(1.0, BORDER),
                StrokeKind::Inside,
            );
            list_ui.painter().text(
                pos2(empty_rect.center().x, empty_rect.center().y - 10.0),
                egui::Align2::CENTER_CENTER,
                "No cards match these filters",
                egui::FontId::proportional(16.0),
                TEXT_BRIGHT,
            );
            list_ui.painter().text(
                pos2(empty_rect.center().x, empty_rect.center().y + 14.0),
                egui::Align2::CENTER_CENTER,
                "Try clearing the search or widening a filter.",
                egui::FontId::proportional(13.0),
                TEXT_DIM,
            );
            return;
        }

        ScrollArea::vertical()
            .id_salt("card_list")
            .show(&mut list_ui, |ui| {
                for entry in &filtered {
                    let (_, available_foils) = self.collection_counts(&entry.name);
                    for is_foil in [false, true] {
                    if is_foil && available_foils == 0 {
                        continue;
                    }
                    let is_site = matches!(entry.zone, Zone::Atlasbook);
                    let (regular_count, foil_count) = self.collection_counts(&entry.name);
                    let printing_owned_count = if is_foil { foil_count } else { regular_count };
                    let is_owned = printing_owned_count > 0;
                    if !ownership_filter.matches_printing(is_owned) {
                        continue;
                    }
                    let map = if is_site {
                        &self.deck_sites
                    } else {
                        &self.deck_spells
                    };
                    let regular_in_deck = map
                        .get(&(entry.name.clone(), false))
                        .copied()
                        .unwrap_or(0);
                    let foil_in_deck = map
                        .get(&(entry.name.clone(), true))
                        .copied()
                        .unwrap_or(0);
                    let total_in_deck = regular_in_deck.saturating_add(foil_in_deck);
                    let current_in_deck = if is_foil { foil_in_deck } else { regular_in_deck };

                    let row_resp =
                        ui.allocate_response(vec2(ui.available_width(), ROW_H), Sense::hover());
                    let row_rect = row_resp.rect;

                    // Track hovered card for preview popup
                    if row_resp.hovered() {
                        let mouse_pos = ctx.input(|i| i.pointer.latest_pos().unwrap_or_default());
                        self.hovered_card = Some((entry.clone(), pos2(mouse_pos.x, mouse_pos.y)));
                    }

                    // Row background — highlight on hover
                    let row_bg = if !is_owned {
                        Color32::from_rgba_premultiplied(18, 20, 29, 180)
                    } else if row_resp.hovered() {
                        Color32::from_rgba_premultiplied(35, 45, 80, 220)
                    } else {
                        Color32::from_rgba_premultiplied(20, 25, 45, 200)
                    };
                    ui.painter()
                        .rect_filled(row_rect, CornerRadius::same(3), row_bg);

                    // Thumbnail
                    let mut image_size = vec2(CARD_THUMB_W, CARD_THUMB_H);
                    let fake_card_data = entry.as_card_data();
                    if fake_card_data.is_site() {
                        image_size = vec2(CARD_THUMB_H, CARD_THUMB_W);
                    }
                    let thumb_rect = Rect::from_min_size(
                        row_rect.min + vec2(4.0, (ROW_H - image_size.y) / 2.0),
                        image_size,
                    );

                    if let Some(tex) = TextureCache::get_card_texture_blocking(&fake_card_data, ctx)
                    {
                        // Paint through a row-clipped painter: card textures can otherwise
                        // bleed a few pixels into the neighbouring list item.
                        ui.painter().with_clip_rect(row_rect).image(
                            tex.id(),
                            thumb_rect,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().rect_filled(
                            thumb_rect,
                            CornerRadius::same(2),
                            Color32::from_rgb(30, 40, 60),
                        );
                        ui.painter().text(
                            thumb_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "?",
                            egui::FontId::proportional(18.0),
                            TEXT_DIM,
                        );
                    }
                    if !is_owned {
                        ui.painter().rect_filled(
                            thumb_rect,
                            CornerRadius::same(2),
                            Color32::from_rgba_premultiplied(8, 10, 16, 150),
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
                        if is_foil {
                            Color32::from_rgb(255, 215, 120)
                        } else if is_owned {
                            TEXT_BRIGHT
                        } else {
                            TEXT_DIM
                        },
                    );
                    if is_foil {
                        ui.painter().rect_stroke(
                            thumb_rect.shrink(0.5),
                            CornerRadius::same(2),
                            Stroke::new(1.0, Color32::from_rgb(255, 215, 120)),
                            StrokeKind::Inside,
                        );
                    }

                    let rarity_color = match entry.rarity {
                        Rarity::Ordinary => Color32::from_rgb(160, 160, 160),
                        Rarity::Exceptional => Color32::from_rgb(80, 180, 255),
                        Rarity::Elite => Color32::from_rgb(200, 160, 60),
                        Rarity::Unique => Color32::from_rgb(200, 80, 200),
                    };
                    let sub_pos = pos2(info_x, row_rect.min.y + 28.0);
                    let card_type_text = entry.card_type.to_string();
                    ui.painter().text(
                        sub_pos,
                        egui::Align2::LEFT_TOP,
                        format!("{card_type_text}  "),
                        egui::FontId::proportional(12.0),
                        if is_owned { TEXT_DIM } else { Color32::from_rgb(100, 105, 120) },
                    );

                    let rarity_x = sub_pos.x
                        + ui.painter()
                            .layout_no_wrap(
                                format!("{card_type_text}  "),
                                egui::FontId::proportional(12.0),
                                TEXT_DIM,
                            )
                            .rect
                            .width();
                    ui.painter().text(
                        pos2(rarity_x, sub_pos.y),
                        egui::Align2::LEFT_TOP,
                        &entry.rarity,
                        egui::FontId::proportional(12.0),
                        rarity_color,
                    );
                    let printing_x = rarity_x
                        + ui.painter()
                            .layout_no_wrap(
                                entry.rarity.to_string(),
                                egui::FontId::proportional(12.0),
                                rarity_color,
                            )
                            .rect
                            .width();
                    ui.painter().text(
                        pos2(printing_x, sub_pos.y),
                        egui::Align2::LEFT_TOP,
                        if is_foil {
                            "  ·  ✦ Foil"
                        } else {
                            "  ·  Standard"
                        },
                        egui::FontId::proportional(12.0),
                        if is_foil {
                            Color32::from_rgb(255, 215, 120)
                        } else if is_owned {
                            TEXT_DIM
                        } else {
                            Color32::from_rgb(100, 105, 120)
                        },
                    );

                    // Mana + thresholds
                    let cost_y = row_rect.min.y + 49.0;
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
                    cx = element_icon::draw_thresholds(
                        ui.painter(),
                        cx,
                        cost_y + 1.0,
                        &entry.thresholds,
                        THRESH_SZ,
                        1.2,
                    );

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

                    // These controls affect this printing only.
                    let btn_w = 24.0;
                    let btn_h = 23.0;
                    let controls_left = row_rect.max.x - 84.0;
                    let controls_y = row_rect.min.y + 29.0;
                    let label_color = if is_foil {
                        Color32::from_rgb(255, 215, 120)
                    } else {
                        TEXT_DIM
                    };
                    ui.painter().text(
                        pos2(controls_left + 36.0, row_rect.min.y + 7.0),
                        egui::Align2::CENTER_TOP,
                        format!("{}/{}", current_in_deck, printing_owned_count),
                        egui::FontId::proportional(10.0),
                        label_color,
                    );
                    let minus_rect = Rect::from_min_size(pos2(controls_left, controls_y), vec2(btn_w, btn_h));
                    let count_rect = Rect::from_min_size(pos2(controls_left + 25.0, controls_y), vec2(34.0, btn_h));
                    let plus_rect = Rect::from_min_size(pos2(controls_left + 60.0, controls_y), vec2(btn_w, btn_h));
                    let can_add = current_in_deck < printing_owned_count
                        && total_in_deck < entry.max_copies();

                    // `allocate_rect` moves the parent layout cursor. Since these rects
                    // live inside the row already allocated above, use `interact` so they
                    // do not pull the next row upward over this card.
                    let minus_response = ui.interact(
                        minus_rect,
                        ui.id().with((
                            "deck_builder_card_action",
                            entry.name.as_str(),
                            is_foil,
                            "minus",
                        )),
                        Sense::click(),
                    );
                    ui.painter().rect_filled(
                        minus_rect,
                        CornerRadius::same(3),
                        if current_in_deck > 0 && minus_response.hovered() {
                            Color32::from_rgb(130, 40, 40)
                        } else {
                            Color32::from_rgb(55, 25, 25)
                        },
                    );
                    ui.painter().text(
                        minus_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "−",
                        egui::FontId::proportional(15.0),
                        Color32::WHITE,
                    );
                    let plus_response = ui.interact(
                        plus_rect,
                        ui.id().with((
                            "deck_builder_card_action",
                            entry.name.as_str(),
                            is_foil,
                            "plus",
                        )),
                        Sense::click(),
                    );
                    ui.painter().rect_filled(
                        plus_rect,
                        CornerRadius::same(3),
                        if can_add && plus_response.hovered() {
                            if is_foil { Color32::from_rgb(145, 105, 28) } else { Color32::from_rgb(30, 110, 50) }
                        } else if can_add {
                            if is_foil { Color32::from_rgb(100, 70, 20) } else { Color32::from_rgb(20, 70, 35) }
                        } else {
                            Color32::from_rgb(25, 30, 25)
                        },
                    );
                    ui.painter().text(
                        count_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{current_in_deck}"),
                        egui::FontId::proportional(14.0),
                        label_color,
                    );
                    ui.painter().text(
                        plus_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(15.0),
                        if can_add { Color32::WHITE } else { Color32::from_rgb(70, 75, 70) },
                    );

                    let delta = if minus_response.clicked() && current_in_deck > 0 {
                        Some(-1)
                    } else if plus_response.clicked() && can_add {
                        Some(1)
                    } else {
                        None
                    };
                    if let Some(delta) = delta {
                        let map = if is_site { &mut self.deck_sites } else { &mut self.deck_spells };
                        let key = (entry.name.clone(), is_foil);
                        let count = map.entry(key.clone()).or_insert(0);
                        if delta > 0 { *count += 1; } else { *count -= 1; }
                        if *count == 0 { map.remove(&key); }
                    }

                    // Separator
                    ui.painter().line_segment(
                        [row_rect.left_bottom(), row_rect.right_bottom()],
                        Stroke::new(0.5, Color32::from_rgb(30, 35, 60)),
                    );

                }
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
        ui.painter().rect_filled(
            name_rect,
            CornerRadius::same(3),
            Color32::from_rgb(18, 22, 40),
        );
        ui.painter().rect_stroke(
            name_rect,
            CornerRadius::same(3),
            egui::Stroke::new(1.0, name_border_col),
            StrokeKind::Outside,
        );
        let te = egui::TextEdit::singleline(&mut self.deck_name)
            .hint_text("Enter deck name…")
            .font(egui::FontId::proportional(13.0))
            .text_color(TEXT_BRIGHT)
            .background_color(Color32::TRANSPARENT)
            .frame(Frame::default());
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
        for (i, entry) in avatars.iter().enumerate() {
            let col = i % avatars_per_row;
            let row_i = i / avatars_per_row;
            let av_rect = Rect::from_min_size(
                pos2(
                    inner.min.x + col as f32 * (avatar_sz.x + 8.0),
                    y + row_i as f32 * (avatar_sz.y + 28.0),
                ),
                avatar_sz,
            );

            let is_selected = self.selected_avatar.as_deref() == Some(&entry.name);
            let is_owned = self.collection.contains_key(&entry.name);
            let av_resp = ui.allocate_rect(av_rect, Sense::click());

            if av_resp.hovered() {
                let mouse_pos = ctx.input(|i| i.pointer.latest_pos().unwrap_or_default());
                self.hovered_card = Some((entry.clone(), pos2(mouse_pos.x, mouse_pos.y)));
            }

            if is_selected {
                ui.painter().rect_stroke(
                    av_rect.expand(2.0),
                    CornerRadius::same(4),
                    Stroke::new(2.0, GOLD),
                    StrokeKind::Outside,
                );
            }

            let fake = entry.as_card_data();
            if let Some(tex) = TextureCache::get_card_texture_blocking(&fake, ctx) {
                egui::Image::new(egui::ImageSource::Texture(
                    egui::load::SizedTexture::from_handle(&tex),
                ))
                .max_size(avatar_sz)
                .paint_at(ui, av_rect);
            } else {
                ui.painter().rect_filled(
                    av_rect,
                    CornerRadius::same(3),
                    Color32::from_rgb(30, 40, 60),
                );
                ui.painter().text(
                    av_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "?",
                    egui::FontId::proportional(18.0),
                    TEXT_DIM,
                );
            }
            if !is_owned {
                ui.painter().rect_filled(
                    av_rect,
                    CornerRadius::same(3),
                    Color32::from_rgba_premultiplied(8, 10, 16, 150),
                );
            }

            // Name below portrait
            let name_pos = pos2(av_rect.center().x, av_rect.max.y + 3.0);
            let av_name_col = if is_selected {
                GOLD
            } else if is_owned {
                TEXT_DIM
            } else {
                Color32::from_rgb(100, 105, 120)
            };
            ui.painter().text(
                name_pos,
                egui::Align2::CENTER_TOP,
                &entry.name,
                egui::FontId::proportional(10.0),
                av_name_col,
            );

            if av_resp.clicked() && is_owned {
                self.selected_avatar = Some(entry.name.clone());
            }
        }

        let rows_count = avatars.len().div_ceil(avatars_per_row);
        y += rows_count as f32 * (avatar_sz.y + 28.0) + 12.0;

        let site_count: u32 = self.deck_sites.values().map(|&c| c as u32).sum();
        let spell_count: u32 = self.deck_spells.values().map(|&c| c as u32).sum();

        // Separator
        ui.painter().line_segment(
            [pos2(inner.min.x, y), pos2(inner.max.x, y)],
            Stroke::new(1.0, BORDER),
        );
        y += 12.0;

        ui.painter().text(
            pos2(inner.min.x, y),
            egui::Align2::LEFT_TOP,
            "Deck progress",
            egui::FontId::proportional(14.0),
            TEXT_BRIGHT,
        );
        y += 22.0;
        for (label, count, required, color) in [
            ("Atlas", site_count, 30, Color32::from_rgb(100, 200, 100)),
            ("Spellbook", spell_count, 60, Color32::from_rgb(120, 160, 255)),
        ] {
            let label_pos = pos2(inner.min.x, y);
            ui.painter().text(
                label_pos,
                egui::Align2::LEFT_TOP,
                format!("{label}  {count}/{required}"),
                egui::FontId::proportional(12.0),
                if count >= required { SUCCESS } else { TEXT_DIM },
            );
            let bar_rect = Rect::from_min_size(
                pos2(inner.min.x + 108.0, y + 2.0),
                vec2((inner.width() - 108.0).max(20.0), 8.0),
            );
            ui.painter().rect_filled(
                bar_rect,
                CornerRadius::same(4),
                Color32::from_rgb(25, 30, 45),
            );
            let fill_width = bar_rect.width() * (count as f32 / required as f32).min(1.0);
            if fill_width > 0.0 {
                ui.painter().rect_filled(
                    Rect::from_min_size(bar_rect.min, vec2(fill_width, bar_rect.height())),
                    CornerRadius::same(4),
                    if count >= required { SUCCESS } else { color },
                );
            }
            y += 22.0;
        }
        y += 4.0;

        // Deck list area
        let deck_list_rect = Rect::from_min_max(
            pos2(inner.min.x, y),
            pos2(inner.max.x, rect.max.y - pad - 30.0),
        );
        let mut deck_ui = ui.new_child(egui::UiBuilder::new().max_rect(deck_list_rect));

        ScrollArea::vertical()
            .id_salt("deck_list")
            .show(&mut deck_ui, |ui| {
                if site_count == 0 && spell_count == 0 {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Your deck is empty")
                            .color(TEXT_BRIGHT)
                            .size(15.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Use the + controls in the collection to add cards. Start with 30 sites and 60 spells.",
                        )
                        .color(TEXT_DIM)
                        .size(13.0),
                    );
                    ui.add_space(14.0);
                }
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

                let mut sites_to_remove: Option<(String, bool)> = None;
                let mut sites_snapshot: Vec<((String, bool), u8)> = self
                    .deck_sites
                    .iter()
                    .map(|(k, &v)| (k.clone(), v))
                    .collect();
                sites_snapshot.sort_by(|a, b| a.0.cmp(&b.0));

                for ((name, is_foil), count) in &sites_snapshot {
                    ui.horizontal(|ui| {
                        if ui.small_button("×").clicked() {
                            sites_to_remove = Some((name.clone(), *is_foil));
                        }
                        ui.label(
                            egui::RichText::new(format!("{count}×  {name}"))
                                .color(TEXT_BRIGHT)
                                .size(13.0),
                        );
                        if *is_foil {
                            ui.label(
                                egui::RichText::new("✦ Foil")
                                    .color(Color32::from_rgb(255, 215, 120))
                                    .size(11.0),
                            );
                        }
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

                let mut spells_to_remove: Option<(String, bool)> = None;
                let mut spells_snapshot: Vec<((String, bool), u8)> = self
                    .deck_spells
                    .iter()
                    .map(|(k, &v)| (k.clone(), v))
                    .collect();
                spells_snapshot.sort_by(|a, b| a.0.cmp(&b.0));

                for ((name, is_foil), count) in &spells_snapshot {
                    ui.horizontal(|ui| {
                        if ui.small_button("×").clicked() {
                            spells_to_remove = Some((name.clone(), *is_foil));
                        }
                        ui.label(
                            egui::RichText::new(format!("{count}×  {name}"))
                                .color(TEXT_BRIGHT)
                                .size(13.0),
                        );
                        if *is_foil {
                            ui.label(
                                egui::RichText::new("✦ Foil")
                                    .color(Color32::from_rgb(255, 215, 120))
                                    .size(11.0),
                            );
                        }
                    });
                }
                if let Some(rm) = spells_to_remove {
                    self.deck_spells.remove(&rm);
                }
            });
    }

    /// Draw a large card preview floating near `anchor`, flipping left/up to stay on screen.
    fn draw_card_preview(ctx: &Context, entry: &CardEntry, anchor: egui::Pos2, screen: Rect) {
        const HTOW_RATIO: f32 = 1.4; // card height-to-width ratio
        const PREVIEW_W: f32 = 270.0;
        const PREVIEW_H: f32 = PREVIEW_W * HTOW_RATIO;
        const PAD: f32 = 8.0;

        // Clamp position so the preview stays fully on screen
        let x = if anchor.x + PREVIEW_W + PAD > screen.max.x {
            anchor.x - PREVIEW_W - PAD * 2.0 // flip to left side of anchor
        } else {
            anchor.x
        };
        let y = (anchor.y)
            .min(screen.max.y - PREVIEW_H - PAD)
            .max(screen.min.y + PAD);

        let mut size = vec2(PREVIEW_W, PREVIEW_H);
        if entry.card_type == CardType::Site {
            size = vec2(PREVIEW_H, PREVIEW_W);
        }
        let preview_rect = Rect::from_min_size(pos2(x, y), size);

        egui::Area::new(egui::Id::new("card_preview_popup"))
            .fixed_pos(preview_rect.min)
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                let fake = entry.as_card_data();
                if let Some(tex) = TextureCache::get_card_texture_blocking(&fake, ctx) {
                    // Drop shadow
                    let shadow_rect = preview_rect.translate(vec2(4.0, 4.0));
                    ui.painter().rect_filled(
                        shadow_rect,
                        CornerRadius::same(6),
                        Color32::from_black_alpha(120),
                    );
                    // Card image
                    ui.painter().rect_stroke(
                        preview_rect,
                        CornerRadius::same(6),
                        Stroke::new(1.5, BORDER),
                        StrokeKind::Outside,
                    );
                    egui::Image::new(egui::ImageSource::Texture(
                        egui::load::SizedTexture::from_handle(&tex),
                    ))
                    .max_size(vec2(PREVIEW_W, PREVIEW_H))
                    .corner_radius(CornerRadius::same(6))
                    .paint_at(ui, preview_rect);
                } else {
                    // Fallback: dark card with name
                    ui.painter().rect_filled(
                        preview_rect,
                        CornerRadius::same(6),
                        Color32::from_rgb(18, 22, 38),
                    );
                    ui.painter().rect_stroke(
                        preview_rect,
                        CornerRadius::same(6),
                        Stroke::new(1.5, BORDER),
                        StrokeKind::Outside,
                    );
                    ui.painter().text(
                        preview_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &entry.name,
                        egui::FontId::proportional(14.0),
                        TEXT_BRIGHT,
                    );
                }
            });
    }
}
