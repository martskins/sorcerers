use super::*;

impl Game {
    pub(super) fn render_gui(&mut self, ui: &mut Ui, painter: &Painter) -> Option<Scene> {
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let is_in_turn = self.current_player == self.data.player_id;
        let is_idle = matches!(self.data.status, Status::Idle);

        let (turn_label, turn_color) = if self.data.status == Status::Mulligan {
            ("SELECT CARDS TO MULLIGAN", theme::TURN_READY)
        } else if is_in_turn {
            ("YOUR TURN", theme::TURN_READY)
        } else {
            ("THEIR TURN", theme::TURN_WAITING)
        };

        let turn_label_pos = pos2(sr.center().x, 28.0);
        painter.text(
            turn_label_pos,
            egui::Align2::CENTER_CENTER,
            turn_label,
            FontId::proportional(16.0),
            turn_color,
        );

        self.render_controls_button(ui, sr);

        #[cfg(debug_assertions)]
        self.render_debug_effects_button(ui, sr);

        #[cfg(debug_assertions)]
        if self.data.show_debug_effects {
            self.render_debug_effects_panel(ui);
        }

        if is_in_turn && is_idle {
            let client = self.client.clone();
            let player_id = self.data.player_id;
            let game_id = self.game_id;
            let btn_size = vec2(160.0, theme::BUTTON_HEIGHT);
            let btn_pos = pos2(sr.max.x - btn_size.x - 18.0, 18.0);
            egui::Area::new(egui::Id::new("pass_turn_btn"))
                .fixed_pos(btn_pos)
                .show(ui, |ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("Pass Turn")
                            .size(18.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(btn_size);
                    if ui.add(btn).clicked() {
                        client
                            .send(ClientMessage::EndTurn { player_id, game_id })
                            .ok();
                    }
                });
        } else if matches!(
            self.data.status,
            Status::SelectingCard { multiple: true, .. }
        ) || self.data.status == Status::Mulligan
        {
            let mut done = false;
            let btn_size = vec2(180.0, theme::BUTTON_HEIGHT);
            let btn_pos = pos2(sr.max.x - btn_size.x - 18.0, 18.0);
            egui::Area::new(egui::Id::new("done_selecting_btn"))
                .fixed_pos(btn_pos)
                .show(ui, |ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("Done Selecting")
                            .size(18.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(btn_size);
                    if ui.add(btn).clicked() {
                        done = true;
                    }
                });
            if done {
                self.broadcast_command(&ComponentCommand::DonePicking);
            }
        }

        let needs_overlay = matches!(
            &self.data.status,
            Status::Waiting { .. }
                | Status::SelectingAction {
                    anchor_on_cursor: false,
                    ..
                }
                | Status::GameAborted { .. }
                | Status::GameOver { .. }
        );
        if needs_overlay {
            painter.rect_filled(
                Rect::from_min_size(pos2(0.0, 0.0), vec2(sr.width(), sr.height())),
                0.0,
                theme::OVERLAY_SCRIM,
            );
        }

        match &self.data.status.clone() {
            Status::Waiting { .. } => None,
            Status::SelectingAmount {
                prompt,
                min_amount,
                max_amount,
                ..
            } => self.render_amount_picker(ui, prompt, *min_amount, *max_amount),
            Status::SelectingAction {
                prompt,
                actions,
                source_card_id,
                anchor_on_cursor,
            } => {
                if Self::is_yes_or_no_actions(actions) {
                    return self.render_yes_or_no_prompt(ui, prompt, *source_card_id);
                }

                let anchor = if *anchor_on_cursor {
                    self.data
                        .last_clicked_cursor_pos
                        .or(self.data.last_clicked_card_pos)
                        .map(|pos| Rect::from_center_size(pos, vec2(1.0, 1.0)))
                } else {
                    None
                };
                let result = popup_action_menu(ui, anchor, prompt, actions, painter);
                if let Some(result) = result {
                    let action_idx = match result {
                        ActionMenuResponse::Selected(idx) => Some(idx),
                        ActionMenuResponse::Dismissed if *anchor_on_cursor => {
                            actions.iter().position(|action| action == "Cancel")
                        }
                        ActionMenuResponse::Dismissed => None,
                    };

                    if let Some(action_idx) = action_idx {
                        self.client
                            .send(ClientMessage::PickAction {
                                game_id: self.game_id,
                                player_id: self.data.player_id,
                                action_idx,
                            })
                            .ok();
                        self.data.status = Status::Idle;
                    }
                }
                None
            }
            Status::GameAborted { reason } => self.render_aborted_window(ui, reason),
            Status::GameOver {
                winner_id,
                winner_name,
            } => self.render_game_over_window(ui, *winner_id, winner_name),
            _ => None,
        }
    }

    fn render_debug_effects_button(&mut self, ui: &mut Ui, sr: Rect) {
        let icon_size = vec2(24.0, 24.0);
        let icon_pos = pos2(sr.center().x + 124.0, 16.0);
        let mut hovered = false;
        egui::Area::new(egui::Id::new("debug_effects_btn"))
            .fixed_pos(icon_pos)
            .order(egui::Order::Foreground)
            .show(ui, |ui| {
                let (rect, response) = ui.allocate_exact_size(icon_size, egui::Sense::click());
                hovered = response.hovered();
                let fill = if self.data.show_debug_effects || hovered {
                    Color32::from_rgb(255, 100, 100)
                } else {
                    Color32::from_rgb(180, 50, 50)
                };
                ui.painter().circle_filled(rect.center(), 12.0, fill);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "D",
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );

                if response.clicked() {
                    self.data.show_debug_effects = !self.data.show_debug_effects;
                }

                response.on_hover_text("Debug Effects (F3)");
            });
    }

    fn render_debug_effects_panel(&mut self, ui: &mut Ui) {
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        egui::Window::new("Effect Debugger")
            .default_pos(pos2(sr.max.x - 320.0, 80.0))
            .default_size(vec2(300.0, 400.0))
            .movable(true)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut self.data.stepped_effects, "Stepped Mode")
                        .changed()
                    {
                        self.client
                            .send(ClientMessage::ToggleSteppedEffects {
                                player_id: self.data.player_id,
                                game_id: self.game_id,
                            })
                            .ok();
                    }

                    if self.data.stepped_effects && ui.button("Step Next").clicked() {
                        self.client
                            .send(ClientMessage::StepNextEffect {
                                player_id: self.data.player_id,
                                game_id: self.game_id,
                            })
                            .ok();
                    }
                });

                ui.separator();
                ui.label(format!("Queue Size: {}", self.data.effect_queue.len()));
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, effect) in self.data.effect_queue.iter().enumerate().rev() {
                            egui::CollapsingHeader::new(format!("{}: {}", i, effect.name))
                                .id_salt(format!("effect_{}", i))
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(
                                        egui::RichText::new(&effect.description).monospace(),
                                    ));
                                });
                        }
                    });
            });
    }

    fn render_controls_button(&mut self, ui: &mut Ui, sr: Rect) {
        let icon_size = vec2(24.0, 24.0);
        let icon_pos = pos2(sr.center().x + 92.0, 16.0);
        let mut hovered = false;
        egui::Area::new(egui::Id::new("controls_help_btn"))
            .fixed_pos(icon_pos)
            .order(egui::Order::Foreground)
            .show(ui, |ui| {
                let (rect, response) = ui.allocate_exact_size(icon_size, egui::Sense::click());
                hovered = response.hovered();
                let fill = if self.data.show_controls_help || hovered {
                    Color32::from_rgba_premultiplied(55, 112, 155, 210)
                } else {
                    Color32::from_rgba_premultiplied(18, 23, 35, 190)
                };
                ui.painter().circle_filled(rect.center(), 11.0, fill);
                ui.painter().circle_stroke(
                    rect.center(),
                    11.0,
                    egui::Stroke::new(1.0, theme::PANEL_BORDER),
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "?",
                    FontId::proportional(15.0),
                    theme::TEXT_BRIGHT,
                );
                if response.on_hover_text("Game controls").clicked() {
                    self.data.show_controls_help = !self.data.show_controls_help;
                }
            });

        if self.data.show_controls_help || hovered {
            let panel_w = 320.0_f32.min(sr.width() - 32.0);
            let panel_pos = pos2((sr.center().x + 122.0).min(sr.max.x - panel_w - 18.0), 48.0);
            egui::Area::new(egui::Id::new("game_controls_help_panel"))
                .fixed_pos(panel_pos)
                .order(egui::Order::Tooltip)
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(theme::PANEL_BG)
                        .stroke(egui::Stroke::new(1.0, theme::PANEL_BORDER))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_width(panel_w - 24.0);
                            ui.label(
                                RichText::new("Controls")
                                    .size(13.0)
                                    .strong()
                                    .color(theme::TEXT_BRIGHT),
                            );
                            ui.add_space(5.0);
                            Self::control_help_row(ui, "Shift + hover", "Preview a card");
                            Self::control_help_row(ui, "Click card", "Select or open actions");
                            Self::control_help_row(ui, "Drag hand card", "Play to the realm");
                            Self::control_help_row(ui, "Effects", "Inspect ongoing effects");
                            Self::control_help_row(ui, "Done Selecting", "Submit multi-picks");
                        });
                });
        }
    }

    fn control_help_row(ui: &mut Ui, input: &str, action: &str) {
        ui.horizontal(|ui| {
            ui.set_height(22.0);
            ui.label(
                RichText::new(input)
                    .size(12.0)
                    .color(Color32::from_rgb(132, 168, 215)),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(action)
                    .size(12.0)
                    .color(Color32::from_rgb(214, 224, 245)),
            );
        });
    }

    fn is_yes_or_no_actions(actions: &[String]) -> bool {
        matches!(actions, [yes, no] if yes == "Yes" && no == "No")
    }

    fn render_yes_or_no_prompt(
        &mut self,
        ui: &mut Ui,
        prompt: &str,
        source_card_id: Option<CardId>,
    ) -> Option<Scene> {
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let card =
            source_card_id.and_then(|id| self.data.cards.iter().find(|c| c.id == id).cloned());
        let has_card = card.is_some();
        let panel_w = (if has_card { 560.0_f32 } else { 380.0_f32 }).min(sr.width() - 32.0);
        let panel_h = (if has_card { 210.0_f32 } else { 168.0_f32 }).min(sr.height() - 32.0);
        let origin = pos2(sr.center().x - panel_w / 2.0, sr.center().y - panel_h / 2.0);
        let mut picked: Option<usize> = None;

        egui::Area::new(egui::Id::new("yes_or_no_prompt"))
            .fixed_pos(origin)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(theme::PANEL_BG)
                    .stroke(egui::Stroke::new(1.0, theme::PANEL_BORDER))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_min_width(panel_w - 32.0);
                        ui.horizontal(|ui| {
                            if let Some(card) = &card {
                                let image_size = vec2(112.0, 112.0 / CARD_ASPECT_RATIO);
                                let (image_rect, _) =
                                    ui.allocate_exact_size(image_size, egui::Sense::hover());
                                if let Some(tex) =
                                    TextureCache::get_card_texture_blocking(card, ui.ctx())
                                {
                                    let mut draw_rect = image_rect;
                                    if tex.aspect_ratio() > 1.0 {
                                        draw_rect = Rect::from_min_size(
                                            image_rect.min,
                                            vec2(image_size.x, image_size.x * CARD_ASPECT_RATIO),
                                        );
                                    }
                                    ui.painter().image(
                                        tex.id(),
                                        draw_rect,
                                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                } else {
                                    ui.painter().rect_filled(
                                        image_rect,
                                        4.0,
                                        Color32::from_rgb(42, 48, 68),
                                    );
                                }
                                ui.add_space(16.0);
                            }

                            let text_w = ui.available_width();
                            ui.vertical(|ui| {
                                ui.set_width(text_w);
                                ui.label(
                                    RichText::new(
                                        card.as_ref()
                                            .map(|card| card.name.as_str())
                                            .unwrap_or("Choose"),
                                    )
                                    .size(18.0)
                                    .strong()
                                    .color(theme::TEXT_BRIGHT),
                                );
                                if has_card {
                                    ui.label(
                                        RichText::new("Triggered ability")
                                            .size(12.0)
                                            .color(Color32::from_rgb(132, 168, 215)),
                                    );
                                }
                                ui.add_space(10.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(prompt)
                                            .size(15.0)
                                            .color(Color32::from_rgb(214, 224, 245)),
                                    )
                                    .wrap(),
                                );
                                ui.add_space(18.0);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let yes = egui::Button::new(
                                            RichText::new("Yes").size(16.0).color(Color32::WHITE),
                                        )
                                        .fill(theme::ACTION)
                                        .min_size(vec2(92.0, 38.0));
                                        if ui.add(yes).clicked() {
                                            picked = Some(0);
                                        }
                                        ui.add_space(8.0);
                                        let no = egui::Button::new(
                                            RichText::new("No").size(16.0).color(Color32::WHITE),
                                        )
                                        .min_size(vec2(92.0, 38.0));
                                        if ui.add(no).clicked() {
                                            picked = Some(1);
                                        }
                                    },
                                );
                            });
                        });
                    });
            });

        if let Some(action_idx) = picked {
            self.client
                .send(ClientMessage::PickAction {
                    game_id: self.game_id,
                    player_id: self.data.player_id,
                    action_idx,
                })
                .ok();
            self.data.status = Status::Idle;
        }

        None
    }

    fn render_amount_picker(
        &mut self,
        ui: &mut Ui,
        prompt: &str,
        min_amount: u8,
        max_amount: u8,
    ) -> Option<Scene> {
        if self.selected_value.is_none() {
            self.selected_value = Some(Box::new(min_amount as i32));
        }

        let Some(selected_amount) = self
            .selected_value
            .as_mut()
            .and_then(|value| value.downcast_mut::<i32>())
        else {
            eprintln!("Amount picker state had unexpected type");
            self.selected_value = Some(Box::new(min_amount as i32));
            return None;
        };
        let mut submitted = false;
        let screen = screen_rect().unwrap_or(Rect::ZERO);
        let panel_w = 420.0_f32.min(screen.width() - 32.0);
        let panel_h = 172.0_f32.min(screen.height() - 32.0);
        let origin = pos2(
            screen.center().x - panel_w / 2.0,
            screen.center().y - panel_h / 2.0,
        );
        egui::Area::new(egui::Id::new("amount_picker_popup"))
            .fixed_pos(origin)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(theme::PANEL_BG)
                    .stroke(egui::Stroke::new(1.0, theme::PANEL_BORDER))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        ui.set_min_width(panel_w - 36.0);
                        ui.set_min_height(panel_h - 36.0);

                        ui.label(
                            RichText::new(prompt)
                                .size(16.0)
                                .strong()
                                .color(theme::TEXT_BRIGHT),
                        );
                        ui.add_space(14.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    *selected_amount > min_amount as i32,
                                    egui::Button::new(
                                        RichText::new("-").size(20.0).color(Color32::WHITE),
                                    )
                                    .min_size(vec2(44.0, 40.0)),
                                )
                                .clicked()
                            {
                                *selected_amount -= 1;
                            }
                            let amt_field = egui::DragValue::new(selected_amount)
                                .range(min_amount as i32..=max_amount as i32)
                                .speed(1)
                                .fixed_decimals(0)
                                .min_decimals(0)
                                .max_decimals(0)
                                .prefix("")
                                .suffix("");
                            ui.add_sized([92.0, 40.0], amt_field);
                            if ui
                                .add_enabled(
                                    *selected_amount < max_amount as i32,
                                    egui::Button::new(
                                        RichText::new("+").size(20.0).color(Color32::WHITE),
                                    )
                                    .min_size(vec2(44.0, 40.0)),
                                )
                                .clicked()
                            {
                                *selected_amount += 1;
                            }
                        });

                        ui.add_space(18.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Submit").size(16.0).color(Color32::WHITE),
                                    )
                                    .fill(theme::ACTION)
                                    .min_size(vec2(112.0, 38.0)),
                                )
                                .clicked()
                            {
                                submitted = true;
                            }
                        });
                });
            });
        if submitted {
            self.client
                .send(ClientMessage::PickAmount {
                    game_id: self.game_id,
                    player_id: self.data.player_id,
                    amount: *selected_amount as u8,
                })
                .ok();
            self.data.status = Status::Idle;
        }
        None
    }

    fn render_aborted_window(&mut self, ui: &mut Ui, reason: &str) -> Option<Scene> {
        let mut new_scene = None;
        egui::Window::new("Game Aborted")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ui, |ui| {
                for line in reason.lines() {
                    ui.label(RichText::new(line).size(12.0));
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Ok").size(18.0).color(Color32::WHITE),
                        )
                        .min_size(vec2(80.0, 24.0)),
                    )
                    .clicked()
                {
                    new_scene = Some(Scene::Menu(Menu::new(self.client.clone())));
                }
            });
        if new_scene.is_some() {
            self.data.status = Status::Idle;
        }
        new_scene
    }

    fn render_game_over_window(
        &mut self,
        ui: &mut Ui,
        winner_id: PlayerId,
        winner_name: &str,
    ) -> Option<Scene> {
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let panel_w = 360.0_f32.min(sr.width() - 32.0);
        let panel_h = 190.0_f32.min(sr.height() - 32.0);
        let origin = pos2(sr.center().x - panel_w / 2.0, sr.center().y - panel_h / 2.0);
        let mut new_scene = None;
        let result = if winner_id == self.data.player_id {
            "Victory"
        } else {
            "Defeat"
        };
        let winner_text = if winner_id == self.data.player_id {
            "You win".to_string()
        } else if winner_name.trim().is_empty() {
            "Opponent wins".to_string()
        } else {
            format!("{winner_name} wins")
        };

        egui::Area::new(egui::Id::new("game_over_window"))
            .fixed_pos(origin)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(theme::PANEL_BG)
                    .stroke(egui::Stroke::new(1.0, theme::PANEL_BORDER))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        ui.set_min_size(vec2(panel_w - 36.0, panel_h - 36.0));
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Game Over")
                                    .size(17.0)
                                    .color(theme::TURN_WAITING),
                            );
                            ui.add_space(8.0);
                            ui.label(RichText::new(result).size(30.0).strong().color(
                                if winner_id == self.data.player_id {
                                    theme::TURN_READY
                                } else {
                                    Color32::from_rgb(224, 96, 104)
                                },
                            ));
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(winner_text.as_str())
                                    .size(16.0)
                                    .color(theme::TEXT_BRIGHT),
                            );
                            ui.add_space(18.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Back to Menu")
                                            .size(15.0)
                                            .color(Color32::WHITE),
                                    )
                                    .min_size(vec2(150.0, 34.0)),
                                )
                                .clicked()
                            {
                                new_scene = Some(Scene::Menu(Menu::new(self.client.clone())));
                            }
                        });
                    });
            });

        new_scene
    }
}
