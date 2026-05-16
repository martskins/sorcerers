use super::*;

impl Game {
    pub(super) fn render_gui(&mut self, ui: &mut Ui, painter: &Painter) -> Option<Scene> {
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let is_in_turn = self.current_player == self.data.player_id;
        let is_idle = matches!(self.data.status, Status::Idle);

        let (turn_label, turn_color) = if is_in_turn {
            ("YOUR TURN", theme::TURN_READY)
        } else {
            ("THEIR TURN", theme::TURN_WAITING)
        };
        let turn_rect = Rect::from_center_size(pos2(sr.center().x, 28.0), vec2(150.0, 34.0));
        painter.rect_filled(
            turn_rect,
            17.0,
            Color32::from_rgba_unmultiplied(13, 18, 28, 220),
        );
        painter.rect_stroke(
            turn_rect,
            17.0,
            Stroke::new(
                1.0,
                if is_in_turn {
                    Color32::from_rgb(72, 155, 95)
                } else {
                    Color32::from_rgb(78, 86, 110)
                },
            ),
            egui::StrokeKind::Outside,
        );
        painter.text(
            turn_rect.center(),
            egui::Align2::CENTER_CENTER,
            turn_label,
            FontId::proportional(16.0),
            turn_color,
        );

        let btn_pos = pos2(sr.max.x - 178.0, sr.max.y - theme::BUTTON_HEIGHT - 12.0);

        if is_in_turn && is_idle {
            let client = self.client.clone();
            let player_id = self.data.player_id;
            let game_id = self.game_id;
            egui::Area::new(egui::Id::new("pass_turn_btn"))
                .fixed_pos(btn_pos)
                .show(ui, |ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("Pass Turn")
                            .size(18.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(vec2(160.0, theme::BUTTON_HEIGHT));
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
            egui::Area::new(egui::Id::new("done_selecting_btn"))
                .fixed_pos(btn_pos)
                .show(ui, |ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("Done Selecting")
                            .size(18.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(vec2(180.0, theme::BUTTON_HEIGHT));
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
            Status::Waiting { .. } | Status::SelectingAction { .. } | Status::GameAborted { .. }
        );
        if needs_overlay {
            painter.rect_filled(
                Rect::from_min_size(pos2(0.0, 0.0), vec2(sr.width(), sr.height())),
                0.0,
                theme::OVERLAY_SCRIM,
            );
        }

        match &self.data.status.clone() {
            Status::Waiting { prompt } => {
                painter.text(
                    sr.center(),
                    egui::Align2::CENTER_CENTER,
                    prompt,
                    FontId::proportional(FONT_SIZE),
                    Color32::WHITE,
                );
                None
            }
            Status::SelectingAmount {
                prompt,
                min_amount,
                max_amount,
            } => self.render_amount_picker(ui, prompt, *min_amount, *max_amount),
            Status::SelectingAction {
                prompt,
                actions,
                anchor_on_cursor,
                ..
            } => {
                let pos = if *anchor_on_cursor {
                    self.data.last_clicked_card_pos
                } else {
                    None
                };
                let result = popup_action_menu(ui, pos, prompt, actions, painter);
                if let Some(idx) = result {
                    self.client
                        .send(ClientMessage::PickAction {
                            game_id: self.game_id,
                            player_id: self.data.player_id,
                            action_idx: idx,
                        })
                        .ok();
                    self.data.status = Status::Idle;
                }
                None
            }
            Status::GameAborted { reason } => self.render_aborted_window(ui, reason),
            _ => None,
        }
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
        let menu_w = 260.0;
        let menu_h = 170.0;
        let screen =
            screen_rect().unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0)));
        let origin = pos2(
            (screen.width() - menu_w) / 2.0,
            (screen.height() - menu_h) / 2.0,
        );
        egui::Area::new(egui::Id::new("amount_picker_popup"))
            .fixed_pos(origin)
            .order(egui::Order::Foreground)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(RichText::new(prompt).size(16.0).color(theme::TEXT_BRIGHT));
                        ui.add_space(18.0);
                        if ui
                            .add_enabled(
                                *selected_amount > min_amount as i32,
                                egui::Button::new("-").min_size(vec2(32.0, 32.0)),
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
                        ui.add_sized([60.0, 32.0], amt_field);
                        if ui
                            .add_enabled(
                                *selected_amount < max_amount as i32,
                                egui::Button::new("+").min_size(vec2(32.0, 32.0)),
                            )
                            .clicked()
                        {
                            *selected_amount += 1;
                        }
                        ui.add_space(18.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Submit").size(18.0).color(Color32::WHITE),
                                )
                                .min_size(vec2(120.0, 36.0)),
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
}
