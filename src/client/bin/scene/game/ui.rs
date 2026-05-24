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
            _ => None,
        }
    }

    fn is_yes_or_no_actions(actions: &[String]) -> bool {
        matches!(actions, [yes, no] if yes == "Yes" && no == "No")
    }

    fn render_yes_or_no_prompt(
        &mut self,
        ui: &mut Ui,
        prompt: &str,
        source_card_id: Option<uuid::Uuid>,
    ) -> Option<Scene> {
        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let card =
            source_card_id.and_then(|id| self.data.cards.iter().find(|c| c.id == id).cloned());
        let has_card = card.is_some();
        let panel_w = (if has_card { 560.0_f32 } else { 380.0_f32 }).min(sr.width() - 32.0);
        let panel_h = (if has_card { 210.0_f32 } else { 168.0_f32 }).min(sr.height() - 32.0);
        let origin = pos2(
            sr.center().x - panel_w / 2.0,
            sr.center().y - panel_h / 2.0,
        );
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
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                                });
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
