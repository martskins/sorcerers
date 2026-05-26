use crate::{
    card_layout::{self, CardDims, HandLayout},
    components::{Component, ComponentCommand, ComponentType},
    render::{self, CardRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{Context, Painter, Pos2, Rect, Sense, Ui, pos2};
use sorcerers::{
    card::CardData,
    game::PlayerId,
    networking::{self, message::ClientMessage},
    zone::Zone,
};

#[derive(Debug)]
pub struct PlayerHandComponent {
    game_id: uuid::Uuid,
    player_id: PlayerId,
    card_rects: Vec<CardRect>,
    client: networking::client::Client,
    visible: bool,
    expanded: bool,
    expansion: f32,
    dragging_card: Option<uuid::Uuid>,
    drag_visual_pos: Option<Pos2>,
    rect: Rect,
    spells_in_hand: Vec<uuid::Uuid>,
    sites_in_hand: Vec<uuid::Uuid>,
}

impl PlayerHandComponent {
    pub fn new(
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        client: networking::client::Client,
        rect: Rect,
    ) -> Self {
        Self {
            game_id: *game_id,
            player_id: *player_id,
            card_rects: Vec::new(),
            client,
            visible: true,
            expanded: false,
            expansion: 0.0,
            dragging_card: None,
            drag_visual_pos: None,
            rect,
            spells_in_hand: Vec::new(),
            sites_in_hand: Vec::new(),
        }
    }

    fn card_height(&self) -> f32 {
        self.rect.height() * 0.8
    }

    fn card_dimensions(&self) -> CardDims {
        CardDims::from_spell_height(self.card_height())
    }

    fn fan_rect_and_rotation(
        &self,
        card_rect: &CardRect,
        index: usize,
        count: usize,
        expansion: f32,
    ) -> (Rect, f32) {
        let count = count.max(1);
        let mid = (count as f32 - 1.0) / 2.0;
        let t = if count == 1 {
            0.0
        } else {
            (index as f32 - mid) / mid.max(1.0)
        };
        let spacing = (card_rect.rect.width() * 0.44)
            .min(self.rect.width() / (count as f32 + 1.2))
            .max(24.0);
        let rotation = t * 0.24;
        let arc = t.abs() * t.abs() * (10.0 + 24.0 * expansion);
        let center_x = self.rect.center().x + (index as f32 - mid) * spacing;
        let collapsed_top = self.rect.max.y - 42.0;
        let expanded_top = self.rect.max.y - card_rect.rect.height() - 12.0;
        let eased = expansion * expansion * (3.0 - 2.0 * expansion);
        let top_y = collapsed_top + (expanded_top - collapsed_top) * eased + arc;
        let rect = Rect::from_min_size(
            pos2(center_x - card_rect.rect.width() / 2.0, top_y),
            card_rect.rect.size(),
        );

        (rect, rotation)
    }

    fn compute_rects(&mut self, cards: &[CardData], ctx: &Context) -> anyhow::Result<()> {
        let spells: Vec<&CardData> = cards
            .iter()
            .filter(|c| c.zone == Zone::Hand && c.owner_id == self.player_id && c.is_spell())
            .collect();

        let sites: Vec<&CardData> = cards
            .iter()
            .filter(|c| c.zone == Zone::Hand && c.owner_id == self.player_id && c.is_site())
            .collect();

        let mut new_spells = spells.len() != self.spells_in_hand.len();
        if !new_spells {
            for spell in &spells {
                if !self.spells_in_hand.contains(&spell.id) {
                    new_spells = true;
                    break;
                }
            }
        }

        let mut new_sites = sites.len() != self.sites_in_hand.len();
        if !new_sites {
            for site in &sites {
                if !self.sites_in_hand.contains(&site.id) {
                    new_sites = true;
                    break;
                }
            }
        }

        if !new_spells && !new_sites {
            // Update textures for existing cards
            for card_rect in &mut self.card_rects {
                if card_rect.image.is_none() {
                    card_rect.image = TextureCache::get_card_texture_blocking(&card_rect.card, ctx);
                }
            }
            return Ok(());
        }

        let spell_count = spells.len();
        let site_count = sites.len();
        let dims = self.card_dimensions();
        let layout = HandLayout::new(spell_count, site_count, dims, self.rect.width() * 0.95);

        let mut rects: Vec<CardRect> = Vec::new();

        for (idx, card) in spells.iter().enumerate() {
            let existing_card = self.card_rects.iter().find(|c| c.card.id == card.id);
            let rect = card_layout::spell_rect(self.rect, &layout, dims, idx);
            rects.push(CardRect {
                rect,
                is_selected: existing_card.is_some_and(|c| c.is_selected),
                image: existing_card
                    .and_then(|c| c.image.clone())
                    .or_else(|| TextureCache::get_card_texture_blocking(card, ctx)),
                card: (*card).clone(),
            });
        }

        if site_count > 0 {
            for (idx, card) in sites.iter().enumerate() {
                let existing_card = self.card_rects.iter().find(|c| c.card.id == card.id);
                let site_rect = card_layout::site_rect(self.rect, &layout, dims, idx);
                let rect = Rect::from_min_size(site_rect.min, dims.spell);
                rects.push(CardRect {
                    rect,
                    is_selected: existing_card.is_some_and(|c| c.is_selected),
                    image: existing_card
                        .and_then(|c| c.image.clone())
                        .or_else(|| TextureCache::get_card_texture_blocking(card, ctx)),
                    card: (*card).clone(),
                });
            }
        }

        self.card_rects = rects;
        self.spells_in_hand = spells.iter().map(|c| c.id).collect();
        self.sites_in_hand = sites.iter().map(|c| c.id).collect();
        Ok(())
    }

    fn card_clicked(&mut self, card_id: &uuid::Uuid, data: &mut GameData) -> anyhow::Result<()> {
        if let Status::SelectingAction { .. } = &data.status {
            return Ok(());
        }

        if let Status::SelectingCard { preview: true, .. } = &data.status {
            return Ok(());
        }

        let mut reset_status = false;
        match &data.status {
            Status::Idle => {
                self.client.send(ClientMessage::ClickCard {
                    card_id: *card_id,
                    player_id: self.player_id,
                    game_id: self.game_id,
                })?;
            }
            Status::SelectingCard {
                cards,
                preview: true,
                ..
            } => {
                if !cards.contains(card_id) {
                    return Ok(());
                }

                self.client.send(ClientMessage::PickCard {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    card_id: *card_id,
                })?;
                reset_status = true;
            }
            Status::SelectingCard {
                cards,
                multiple: false,
                preview: false,
                ..
            } => {
                if !cards.contains(card_id) {
                    return Ok(());
                }

                self.client.send(ClientMessage::PickCard {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    card_id: *card_id,
                })?;
                reset_status = true;
            }
            Status::Mulligan => {
                if let Some(card) = self.card_rects.iter_mut().find(|c| c.card.id == *card_id) {
                    card.is_selected = !card.is_selected;
                }
            }
            _ => {}
        }

        if reset_status {
            data.status = Status::Idle;
        }

        Ok(())
    }
}

impl Component for PlayerHandComponent {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        self.compute_rects(&data.cards, ctx)?;
        Ok(())
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let pointer = ui.ctx().pointer_latest_pos();
        let collapsed_height = 46.0;
        let collapsed_strip = Rect::from_min_max(
            pos2(self.rect.min.x, self.rect.max.y - collapsed_height),
            self.rect.max,
        );
        let hand_cards: Vec<&CardRect> = self
            .card_rects
            .iter()
            .filter(|card| card.card.zone == Zone::Hand)
            .collect();
        let dragging = self.dragging_card.is_some();
        let hover_card = pointer.is_some_and(|pos| {
            hand_cards.iter().enumerate().any(|(idx, card_rect)| {
                let (rect, _) =
                    self.fan_rect_and_rotation(card_rect, idx, hand_cards.len(), self.expansion);
                let visible_rect = if self.expansion < 0.05 {
                    rect.intersect(collapsed_strip)
                } else {
                    rect
                };
                visible_rect.contains(pos)
            })
        });

        self.expanded = hover_card || dragging;

        let target = if self.expanded { 1.0 } else { 0.0 };
        let step = ui.ctx().input(|i| i.stable_dt).max(1.0 / 120.0) * 9.0;
        if self.expansion < target {
            self.expansion = (self.expansion + step).min(target);
            ui.ctx().request_repaint();
        } else if self.expansion > target {
            self.expansion = (self.expansion - step).max(target);
            ui.ctx().request_repaint();
        }

        if self.expansion <= 0.01 && !dragging {
            let clipped = painter.with_clip_rect(collapsed_strip);
            for (idx, card_rect) in hand_cards.iter().enumerate() {
                let mut collapsed_card = (*card_rect).clone();
                let (rect, rotation) =
                    self.fan_rect_and_rotation(&collapsed_card, idx, hand_cards.len(), 0.0);
                collapsed_card.rect = rect;
                render::draw_card_with_texture_rotation(
                    &collapsed_card,
                    true,
                    false,
                    &clipped,
                    rotation,
                    collapsed_card.card.is_site(),
                );
            }
            return Ok(None);
        }

        let mut clicked_card: Option<(uuid::Uuid, Rect, Pos2)> = None;
        let mut dropped_card: Option<(uuid::Uuid, egui::Pos2)> = None;
        let suppress_preview = matches!(data.status, Status::SelectingAction { .. });
        let drag_painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("dragged_hand_card"),
        ));
        for (idx, card_rect) in hand_cards.iter().enumerate() {
            let (rect, rotation) =
                self.fan_rect_and_rotation(card_rect, idx, hand_cards.len(), self.expansion);
            let mut fan_card = (*card_rect).clone();
            fan_card.rect = rect;

            let resp = ui.allocate_rect(fan_card.rect, Sense::HOVER | Sense::CLICK | Sense::DRAG);
            let is_dragging_this_card = self.dragging_card == Some(fan_card.card.id);
            let draw_rect = if is_dragging_this_card {
                pointer
                    .map(|pos| Rect::from_center_size(pos, fan_card.rect.size()))
                    .unwrap_or(fan_card.rect)
            } else if resp.dragged() {
                fan_card.rect.translate(resp.drag_delta())
            } else {
                fan_card.rect
            };
            if !is_dragging_this_card {
                fan_card.rect = draw_rect;
                render::draw_card_with_texture_rotation(
                    &fan_card,
                    true,
                    false,
                    painter,
                    rotation,
                    fan_card.card.is_site(),
                );
            }
            if resp.clicked() {
                let click_pos = resp
                    .interact_pointer_pos()
                    .unwrap_or(fan_card.rect.center());
                clicked_card = Some((fan_card.card.id, fan_card.rect, click_pos));
            }

            if resp.drag_started() {
                self.dragging_card = Some(fan_card.card.id);
                self.drag_visual_pos = pointer;
                self.client.send(ClientMessage::RequestPlayableZones {
                    card_id: fan_card.card.id,
                    player_id: self.player_id,
                    game_id: self.game_id,
                })?;
            }

            if resp.drag_stopped() && self.dragging_card == Some(fan_card.card.id) {
                if let Some(pos) = ui.ctx().pointer_latest_pos() {
                    dropped_card = Some((fan_card.card.id, pos));
                }
                self.dragging_card = None;
                self.drag_visual_pos = None;
            }

            if is_dragging_this_card {
                if let Some(target) = pointer {
                    let dt = ui.ctx().input(|i| i.stable_dt).max(1.0 / 120.0);
                    let current = self.drag_visual_pos.unwrap_or(target);
                    let alpha = (dt * 18.0).clamp(0.0, 1.0);
                    self.drag_visual_pos = Some(current + (target - current) * alpha);
                }
                let center = self
                    .drag_visual_pos
                    .or(pointer)
                    .unwrap_or(draw_rect.center());
                let floating_size = draw_rect.size() * 1.04;
                fan_card.rect = Rect::from_center_size(center, floating_size);
                render::draw_card_with_texture_rotation(
                    &fan_card,
                    true,
                    false,
                    &drag_painter,
                    0.0,
                    fan_card.card.is_site(),
                );
                ui.ctx().request_repaint();
            } else if resp.hovered() && !resp.clicked() && !suppress_preview {
                render::draw_card_preview(ui, fan_card.image.as_ref())?;
            }
        }

        if let Some((card_id, pos)) = dropped_card {
            return Ok(Some(ComponentCommand::DropHandCard { card_id, pos }));
        }

        if let Some((card_id, card_rect, click_pos)) = clicked_card {
            // Track the click origin so realm.rs can animate the card flying to its zone.
            if matches!(data.status, Status::Idle) {
                data.last_clicked_card_id = Some(card_id);
                data.last_clicked_card_pos = Some(card_rect.center());
                data.last_clicked_card_rect = Some(card_rect);
                data.last_clicked_cursor_pos = Some(click_pos);
                data.last_clicked_card_time = Some(ui.ctx().input(|i| i.time));
            }
            self.card_clicked(&card_id, data)?;
        }

        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn process_command(
        &mut self,
        command: &ComponentCommand,
        data: &mut GameData,
    ) -> anyhow::Result<()> {
        match command {
            ComponentCommand::DonePicking if matches!(data.status, Status::Mulligan) => {
                let selected_cards: Vec<uuid::Uuid> = self
                    .card_rects
                    .iter()
                    .filter(|c| c.is_selected)
                    .map(|c| c.card.id)
                    .collect();
                self.client.send(ClientMessage::PickCards {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    card_ids: selected_cards,
                })?;
                data.status = Status::Idle;
            }
            ComponentCommand::SetRect {
                component_type: ComponentType::PlayerHand,
                rect,
            } => {
                self.rect = *rect;
            }
            _ => {}
        }
        Ok(())
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::PlayerHand
    }
}
