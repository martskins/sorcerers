use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{card_height, card_width},
    input::Mouse,
    render::{self, CardRect},
    scene::game::GameData,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{Color, WHITE},
    input::{MouseButton, is_mouse_button_released},
    math::{Rect, RectOffset, Vec2},
    shapes::draw_rectangle,
    ui,
    window::{screen_height, screen_width},
};
use sorcerers::{
    card::CardData,
    game::PlayerId,
    networking::{self, message::ClientMessage},
};

const FONT_SIZE: f32 = 24.0;

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionOverlayBehaviour {
    Preview,
    Pick,
}

#[derive(Debug)]
pub struct SelectionOverlay {
    card_rects: Vec<CardRect>,
    prompt: String,
    behaviour: SelectionOverlayBehaviour,
    close: bool,
    player_id: PlayerId,
    game_id: uuid::Uuid,
    client: networking::client::Client,
}

impl SelectionOverlay {
    pub async fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        cards: Vec<&CardData>,
        prompt: &str,
        behaviour: SelectionOverlayBehaviour,
    ) -> anyhow::Result<Self> {
        let mut textures = Vec::with_capacity(cards.len());
        for card in &cards {
            let texture = TextureCache::get_card_texture(card).await?;
            textures.push(texture);
        }

        let card_spacing = 20.0;
        let card_count = cards.len();
        let card_width = card_width()? * 2.0;
        let card_height = card_height()? * 2.0;
        let cards_area_width = card_count as f32 * card_width + (card_count as f32 - 1.0) * card_spacing;
        let cards_start_x = (screen_width() - cards_area_width) / 2.0;
        let cards_y = (screen_height() - card_height) / 2.0 + 30.0;

        let mut rects = Vec::with_capacity(cards.len());
        for (idx, card) in cards.into_iter().enumerate() {
            let mut size = Vec2::new(card_width, card_height);
            if card.is_site() {
                size = Vec2::new(card_height, card_width);
            }
            let x = cards_start_x + idx as f32 * (size.x + card_spacing);
            let rect = CardRect {
                image: textures[idx].clone(),
                rect: Rect::new(x, cards_y, size.x, size.y),
                is_hovered: false,
                is_selected: false,
                card: card.clone(),
            };
            rects.push(rect);
        }

        Ok(Self {
            client,
            game_id: game_id.clone(),
            card_rects: rects,
            prompt: prompt.to_string(),
            behaviour,
            player_id: player_id.clone(),
            close: false,
        })
    }
}

#[async_trait::async_trait]
impl Component for SelectionOverlay {
    async fn update(&mut self, _data: &mut GameData) -> anyhow::Result<()> {
        if is_mouse_button_released(MouseButton::Left) {
            Mouse::set_enabled(true)?;
        }

        let mouse_pos: Vec2 = macroquad::input::mouse_position().into();
        for cell_rect in &mut self.card_rects {
            cell_rect.is_hovered = cell_rect.rect.contains(mouse_pos);
        }

        Ok(())
    }

    async fn process_command(&mut self, _command: &ComponentCommand, _data: &mut GameData) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self) {}

    fn is_visible(&self) -> bool {
        true
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::SelectionOverlay
    }

    async fn process_input(
        &mut self,
        _in_turn: bool,
        _data: &mut GameData,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let mouse_position = macroquad::input::mouse_position();
        let mouse_vec = Vec2::new(mouse_position.0, mouse_position.1);

        for rect in &mut self.card_rects {
            if !Mouse::enabled()? {
                continue;
            }

            if rect.rect.contains(mouse_vec) && is_mouse_button_released(MouseButton::Left) {
                match self.behaviour {
                    SelectionOverlayBehaviour::Preview => {
                        self.client.send(ClientMessage::ClickCard {
                            game_id: self.game_id.clone(),
                            player_id: self.player_id.clone(),
                            card_id: rect.card.id.clone(),
                        })?;
                        self.close = true;
                    }
                    SelectionOverlayBehaviour::Pick => {
                        self.client.send(ClientMessage::PickCard {
                            game_id: self.game_id.clone(),
                            player_id: self.player_id.clone(),
                            card_id: rect.card.id.clone(),
                        })?;
                        self.close = true;
                    }
                }
            }
        }

        Ok(None)
    }

    async fn render(&mut self, _data: &mut GameData) -> anyhow::Result<()> {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.8),
        );

        let window_style = ui::root_ui()
            .style_builder()
            .background_margin(RectOffset::new(10.0, 10.0, 10.0, 10.0))
            .build();
        let skin = ui::Skin {
            window_style,
            ..ui::root_ui().default_skin()
        };

        ui::root_ui().push_skin(&skin);

        let card_count = self.card_rects.len();
        let card_width = card_width()? * 2.0;
        let card_height = card_height()? * 2.0;
        let card_spacing = 20.0;

        let mut skin = ui::root_ui().default_skin();
        skin.button_style = ui::root_ui()
            .style_builder()
            .color(Color::new(0.0, 0.0, 0.0, 0.0))
            .build();
        skin.label_style = ui::root_ui().style_builder().font_size(FONT_SIZE as u16).build();
        ui::root_ui().push_skin(&skin);

        let cards_area_width = card_count as f32 * card_width + (card_count as f32 - 1.0) * card_spacing;
        let cards_start_x = (screen_width() - cards_area_width) / 2.0;
        let cards_y = (screen_height() - card_height) / 2.0 + 30.0;

        let wrapped_text = render::wrap_text(&self.prompt, screen_width() - 20.0, FONT_SIZE as u16);
        macroquad::text::draw_multiline_text(
            &wrapped_text,
            cards_start_x - 50.0,
            cards_y - 50.0,
            FONT_SIZE,
            Some(1.0),
            WHITE,
        );

        let mut rects = self.card_rects.clone();
        rects.sort_by_key(|f| f.is_hovered);
        for card_rect in &self.card_rects {
            render::draw_card(card_rect, card_rect.card.controller_id == self.player_id, false);
        }

        if self.behaviour == SelectionOverlayBehaviour::Preview {
            let close_button_pos = Vec2::new(screen_width() / 2.0 - 50.0, cards_y + card_height + 20.0);
            let close_button_size = Vec2::new(100.0, 40.0);
            let close_button = ui::widgets::Button::new("Close")
                .position(close_button_pos)
                .size(close_button_size)
                .ui(&mut ui::root_ui());
            if close_button {
                self.close = true;
            }
        }

        ui::root_ui().pop_skin();
        Ok(())
    }
}
