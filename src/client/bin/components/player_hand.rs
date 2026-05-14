use crate::{
    card_layout::{self, CardDims, HandLayout},
    components::{Component, ComponentCommand, ComponentType},
    render::{self, CardRect},
    scene::game::{GameData, Status},
    theme,
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Sense, Stroke, Ui, vec2};
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
                let rect = card_layout::site_rect(self.rect, &layout, dims, idx);
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
        painter.rect_filled(self.rect, 0.0, theme::HAND_BG);
        painter.rect_filled(
            self.rect.shrink2(vec2(10.0, 8.0)),
            8.0,
            Color32::from_rgba_unmultiplied(26, 32, 43, 190),
        );
        painter.line_segment(
            [self.rect.left_top(), self.rect.right_top()],
            Stroke::new(1.0, Color32::from_rgb(70, 84, 110)),
        );

        let mut clicked_card: Option<(uuid::Uuid, egui::Pos2)> = None;
        for card_rect in &self.card_rects {
            if card_rect.card.zone != Zone::Hand {
                continue;
            }

            let resp = ui.allocate_rect(card_rect.rect, Sense::HOVER | Sense::CLICK);
            render::draw_card(card_rect, true, false, painter);
            if resp.clicked() {
                clicked_card = Some((card_rect.card.id, card_rect.rect.center()));
            }

            if resp.hovered() {
                render::draw_card_preview(ui, card_rect.image.as_ref())?;
            }
        }

        if let Some((card_id, card_center)) = clicked_card {
            // Track the click origin so realm.rs can animate the card flying to its zone.
            if matches!(data.status, Status::Idle) {
                data.last_clicked_card_id = Some(card_id);
                data.last_clicked_card_pos = Some(card_center);
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
