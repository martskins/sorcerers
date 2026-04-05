use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    input::Mouse,
    render::{self, CardRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Ui, pos2, vec2};
use sorcerers::{
    card::{CardData, Zone},
    networking::{self, message::ClientMessage},
};

#[derive(Debug)]
pub struct PlayerHandComponent {
    game_id: uuid::Uuid,
    player_id: uuid::Uuid,
    card_rects: Vec<CardRect>,
    client: networking::client::Client,
    visible: bool,
    rect: Rect,
    spells_in_hand: Vec<uuid::Uuid>,
    sites_in_hand: Vec<uuid::Uuid>,
}

impl PlayerHandComponent {
    pub fn new(game_id: &uuid::Uuid, player_id: &uuid::Uuid, client: networking::client::Client, rect: Rect) -> Self {
        Self {
            game_id: game_id.clone(),
            player_id: player_id.clone(),
            card_rects: Vec::new(),
            client,
            visible: true,
            rect,
            spells_in_hand: Vec::new(),
            sites_in_hand: Vec::new(),
        }
    }

    fn card_width(&self) -> f32 {
        self.card_height() * CARD_ASPECT_RATIO
    }

    fn card_height(&self) -> f32 {
        self.rect.height() * 0.8
    }

    fn spell_dimensions(&self) -> egui::Vec2 {
        vec2(self.card_width(), self.card_height())
    }

    pub fn site_dimensions(&self) -> egui::Vec2 {
        vec2(self.card_height(), self.card_width())
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
        let spell_dim = self.spell_dimensions();
        let site_dim = self.site_dimensions();

        let min_visible_width = spell_dim.x * 0.25;
        let max_hand_width = self.rect.width() * 0.95;
        let spell_spacing = if spell_count > 1 {
            ((max_hand_width - spell_dim.x) / (spell_count as f32 - 1.0))
                .min(spell_dim.x - min_visible_width)
                .max(10.0)
        } else {
            0.0
        };

        let spells_width = if spell_count > 0 {
            spell_dim.x + (spell_count as f32 - 1.0) * spell_spacing
        } else {
            0.0
        };

        let sites_per_column = 4;
        let site_columns = ((site_count + sites_per_column - 1) / sites_per_column).max(1);
        let site_spacing_y = (site_dim.y * 0.15).max(20.0);
        let site_spacing_x = 20.0;

        let sites_width = if site_count > 0 {
            site_columns as f32 * site_dim.x + (site_columns as f32 - 1.0) * site_spacing_x
        } else {
            0.0
        };

        let total_width = spells_width + if site_count > 0 { site_spacing_x + sites_width } else { 0.0 };
        let start_x = self.rect.min.x + (self.rect.width() - total_width) / 2.0;
        let spells_y = self.rect.min.y + self.rect.height() / 2.0 - spell_dim.y / 2.0;

        let mut rects: Vec<CardRect> = Vec::new();

        for (idx, card) in spells.iter().enumerate() {
            let existing_card = self.card_rects.iter().find(|c| c.card.id == card.id);
            let x = start_x + idx as f32 * spell_spacing;
            let rect = Rect::from_min_size(pos2(x, spells_y), spell_dim);
            rects.push(CardRect {
                rect,
                is_hovered: existing_card.map_or(false, |c| c.is_hovered),
                is_selected: existing_card.map_or(false, |c| c.is_selected),
                image: existing_card
                    .and_then(|c| c.image.clone())
                    .or_else(|| TextureCache::get_card_texture_blocking(card, ctx)),
                card: (*card).clone(),
            });
        }

        if site_count > 0 {
            let sites_x = start_x + spells_width + site_spacing_x;
            let sites_start_y = self.rect.min.y + self.rect.height() / 2.0 - spell_dim.y / 2.0;
            for (idx, card) in sites.iter().enumerate() {
                let existing_card = self.card_rects.iter().find(|c| c.card.id == card.id);
                let col = idx / sites_per_column;
                let row = idx % sites_per_column;
                let x = sites_x + col as f32 * (site_dim.x + site_spacing_x);
                let y = sites_start_y + row as f32 * site_spacing_y;
                let rect = Rect::from_min_size(pos2(x, y), site_dim);
                rects.push(CardRect {
                    rect,
                    is_hovered: existing_card.map_or(false, |c| c.is_hovered),
                    is_selected: existing_card.map_or(false, |c| c.is_selected),
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

    fn render_card_preview(&self, data: &mut GameData, painter: &Painter) -> anyhow::Result<()> {
        if let Some(card) = self.card_rects.iter().find(|card| card.is_hovered) {
            render::render_card_preview(card, data, painter)?;
        }
        Ok(())
    }
}

impl Component for PlayerHandComponent {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        self.compute_rects(&data.cards, ctx)?;
        Ok(())
    }

    fn render(&mut self, data: &mut GameData, _ui: &mut Ui, painter: &Painter) -> anyhow::Result<()> {
        let bg_color = Color32::from_rgba_unmultiplied(38, 46, 56, 217);
        painter.rect_filled(self.rect, 0.0, bg_color);

        for card_rect in &self.card_rects {
            if card_rect.card.zone != Zone::Hand {
                continue;
            }
            render::draw_card(card_rect, true, false, painter);
        }

        self.render_card_preview(data, painter)?;
        Ok(())
    }

    fn process_input(&mut self, in_turn: bool, data: &mut GameData, ctx: &Context) -> anyhow::Result<Option<ComponentCommand>> {
        if !Mouse::enabled() {
            return Ok(None);
        }

        if let Status::SelectingAction { .. } = &data.status {
            return Ok(None);
        }

        if !in_turn && Status::Mulligan != data.status {
            return Ok(None);
        }

        let mouse_pos = Mouse::position(ctx);

        let mut hovered_card_index = None;
        if let Some(mp) = mouse_pos {
            for (idx, card_display) in self.card_rects.iter().enumerate() {
                if card_display.rect.contains(mp) {
                    hovered_card_index = Some(idx);
                }
            }
        }

        for card in &mut self.card_rects {
            card.is_hovered = false;
        }
        if let Some(idx) = hovered_card_index {
            self.card_rects
                .get_mut(idx)
                .ok_or(anyhow::anyhow!("expected to find rect"))?
                .is_hovered = true;
        }

        let clicked = Mouse::clicked(ctx);
        let mp = mouse_pos.unwrap_or_default();

        match &data.status {
            Status::Idle => {
                for card_rect in self.card_rects.iter().filter(|c| c.card.zone.is_in_play() || c.card.zone == Zone::Hand) {
                    if card_rect.is_hovered && clicked {
                        self.client.send(ClientMessage::ClickCard {
                            card_id: card_rect.card.id,
                            player_id: self.player_id,
                            game_id: self.game_id,
                        })?;
                    }
                }
            }
            Status::SelectingCard { cards, preview: true, .. } => {
                let mut selected_id = None;
                for card_rect in self.card_rects.iter().filter(|c| cards.contains(&c.card.id)) {
                    if card_rect.rect.contains(mp) && clicked {
                        selected_id = Some(card_rect.card.id);
                    }
                }
                if let Some(id) = selected_id {
                    self.client.send(ClientMessage::PickCard {
                        player_id: self.player_id,
                        game_id: self.game_id,
                        card_id: id,
                    })?;
                    data.status = Status::Idle;
                }
            }
            Status::SelectingCard { cards, multiple: false, preview: false, .. } => {
                let mut selected_id = None;
                for card_rect in self.card_rects.iter().filter(|c| cards.contains(&c.card.id)) {
                    if card_rect.rect.contains(mp) && clicked {
                        selected_id = Some(card_rect.card.id);
                    }
                }
                if let Some(id) = selected_id {
                    self.client.send(ClientMessage::PickCard {
                        player_id: self.player_id,
                        game_id: self.game_id,
                        card_id: id,
                    })?;
                    data.status = Status::Idle;
                }
            }
            Status::Mulligan => {
                let mut selected_id = None;
                for card_rect in self.card_rects.iter().filter(|c| c.card.zone == Zone::Hand) {
                    if card_rect.rect.contains(mp) && clicked {
                        selected_id = Some(card_rect.card.id);
                    }
                }
                if let Some(id) = selected_id {
                    if let Some(card) = self.card_rects.iter_mut().find(|c| c.card.id == id) {
                        card.is_selected = !card.is_selected;
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn process_command(&mut self, command: &ComponentCommand, data: &mut GameData) -> anyhow::Result<()> {
        match command {
            ComponentCommand::DonePicking if matches!(data.status, Status::Mulligan) => {
                let selected_cards: Vec<uuid::Uuid> =
                    self.card_rects.iter().filter(|c| c.is_selected).map(|c| c.card.id).collect();
                self.client.send(ClientMessage::PickCards {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    card_ids: selected_cards,
                })?;
                data.status = Status::Idle;
            }
            ComponentCommand::SetRect { component_type: ComponentType::PlayerHand, rect } => {
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
