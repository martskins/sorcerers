use std::collections::HashMap;

use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{card_height, card_width, screen_rect},
    render::{self, CardRect},
    scene::game::GameData,
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Ui, pos2, vec2};
use sorcerers::{
    card::CardData,
    game::PlayerId,
    networking::{self, message::ClientMessage},
};

const FONT_SIZE: f32 = 24.0;

#[derive(Debug)]
pub struct CombatResolutionOverlay {
    card_rects: Vec<CardRect>,
    prompt: String,
    player_id: PlayerId,
    game_id: uuid::Uuid,
    client: networking::client::Client,
    defender_damage: HashMap<uuid::Uuid, u16>,
    damage: u16,
    shake_button_until: Option<chrono::DateTime<chrono::Utc>>,
    visible: bool,
}

impl CombatResolutionOverlay {
    pub fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        attacker: CardData,
        defenders: Vec<CardData>,
        damage: u16,
    ) -> Self {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        let cw = card_width().unwrap_or(80.0) * 1.2;
        let ch = card_height().unwrap_or(112.0) * 1.2;
        let card_spacing = 20.0;

        let attacker_x = (sw - cw) / 2.0;
        let attacker_y = (sh / 3.0) - (ch / 2.0);

        let mut rects = Vec::new();
        rects.push(CardRect {
            rect: Rect::from_min_size(pos2(attacker_x, attacker_y), vec2(cw, ch)),
            card: attacker,
            image: None,
            is_selected: false,
        });

        let defender_count = defenders.len();
        let defenders_area_width =
            defender_count as f32 * cw + (defender_count as f32 - 1.0) * card_spacing;
        let defenders_start_x = (sw - defenders_area_width) / 2.0;
        let defenders_y = (3.0 * sh / 5.0) - (ch / 2.0);

        for (idx, defender) in defenders.iter().enumerate() {
            let x = defenders_start_x + idx as f32 * (cw + card_spacing);
            rects.push(CardRect {
                rect: Rect::from_min_size(pos2(x, defenders_y), vec2(cw, ch)),
                card: defender.clone(),
                image: None,
                is_selected: false,
            });
        }

        Self {
            client,
            game_id: game_id.clone(),
            card_rects: rects,
            prompt: format!("Distribute {} damage among defenders", damage),
            player_id: player_id.clone(),
            defender_damage: HashMap::new(),
            damage,
            shake_button_until: None,
            visible: true,
        }
    }
}

impl Component for CombatResolutionOverlay {
    fn update(&mut self, _data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        for card_rect in &mut self.card_rects {
            if card_rect.image.is_none() {
                card_rect.image = TextureCache::get_card_texture_blocking(&card_rect.card, ctx);
            }
        }
        Ok(())
    }

    fn process_command(
        &mut self,
        _command: &ComponentCommand,
        _data: &mut GameData,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self) {}

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::CombatResolutionOverlay
    }

    fn render(
        &mut self,
        _data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        painter.rect_filled(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(sw, sh)),
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 204),
        );

        painter.text(
            pos2(sw / 2.0, 30.0),
            egui::Align2::CENTER_TOP,
            &self.prompt,
            egui::FontId::proportional(FONT_SIZE),
            Color32::WHITE,
        );

        for card_rect in &self.card_rects {
            render::draw_card(
                card_rect,
                card_rect.card.controller_id == self.player_id,
                false,
                painter,
            );
        }

        // Damage controls for defenders (skip first card = attacker)
        let damage_assigned: u16 = self.defender_damage.values().sum();
        for card_rect in self.card_rects.iter().skip(1) {
            if card_rect.card.controller_id != self.player_id {
                let card_id = card_rect.card.id;
                let damage = self.defender_damage.get(&card_id).copied().unwrap_or(0);
                let label_x = card_rect.rect.min.x + card_rect.rect.width() / 2.0;
                let label_y = card_rect.rect.max.y + 10.0;

                let mut dmg_delta: i16 = 0;
                egui::Area::new(egui::Id::new(format!("combat_dmg_{}", card_id)))
                    .fixed_pos(pos2(label_x - 60.0, label_y))
                    .show(ui.ctx(), |ui| {
                        ui.horizontal(|ui| {
                            let minus = egui::Button::new(
                                egui::RichText::new("−").size(20.0).color(Color32::WHITE),
                            )
                            .min_size(vec2(36.0, 36.0));
                            if ui.add(minus).clicked() && damage > 0 {
                                dmg_delta = -1;
                            }
                            ui.label(
                                egui::RichText::new(damage.to_string())
                                    .size(20.0)
                                    .color(Color32::WHITE),
                            );
                            let plus = egui::Button::new(
                                egui::RichText::new("+").size(20.0).color(Color32::WHITE),
                            )
                            .min_size(vec2(36.0, 36.0));
                            if ui.add(plus).clicked() && damage_assigned < self.damage {
                                dmg_delta = 1;
                            }
                        });
                    });
                if dmg_delta != 0 {
                    let new_dmg = (damage as i16 + dmg_delta).max(0) as u16;
                    self.defender_damage.insert(card_id, new_dmg);
                }
            }
        }

        let defender_row_y = self
            .card_rects
            .iter()
            .skip(1)
            .map(|r| r.rect.max.y)
            .fold(0.0_f32, f32::max);
        let button_width = 150.0;
        let button_y = defender_row_y + 90.0; // extra space for damage controls
        let mut button_x = (sw - button_width) / 2.0;

        if let Some(shake_until) = self.shake_button_until {
            let now = chrono::Utc::now();
            if now < shake_until {
                let elapsed = (shake_until - now).num_milliseconds() as f32;
                let shake_magnitude = 3.0 * (elapsed / 300.0);
                let shake_x = (rand::random::<f32>() * 2.0 - 1.0) * shake_magnitude;
                button_x += shake_x;
            } else {
                self.shake_button_until = None;
            }
        }

        let current_damage = self.damage;
        let client = self.client.clone();
        let game_id = self.game_id;
        let player_id = self.player_id;
        let defender_damage = self.defender_damage.clone();
        let mut set_invisible = false;
        let mut set_shake = false;

        egui::Area::new(egui::Id::new("combat_confirm_btn"))
            .fixed_pos(pos2(button_x, button_y))
            .show(ui.ctx(), |ui| {
                let confirm = egui::Button::new(
                    egui::RichText::new("Confirm")
                        .size(22.0)
                        .color(Color32::WHITE),
                )
                .min_size(vec2(button_width, 48.0));
                if ui.add(confirm).clicked() {
                    let assigned: u16 = defender_damage.values().sum();
                    if assigned != current_damage {
                        set_shake = true;
                    } else {
                        client
                            .send(ClientMessage::ResolveCombat {
                                game_id,
                                player_id,
                                damage_assignment: defender_damage,
                            })
                            .ok();
                        set_invisible = true;
                    }
                }
            });

        if set_shake {
            self.shake_button_until =
                Some(chrono::Utc::now() + chrono::Duration::milliseconds(300));
        }
        if set_invisible {
            self.visible = false;
        }

        Ok(None)
    }
}
