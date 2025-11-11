use std::sync::{Arc, Mutex};

use macroquad::prelude::*;
use sorcerers::networking::{self, Message};

use crate::scene::menu::Menu;
use crate::scene::Scene;

#[derive(Debug)]
pub struct Client {
    scene: Arc<Mutex<Scene>>,
    client: networking::client::Client,
}

impl Client {
    pub fn new() -> anyhow::Result<Self> {
        let client = networking::client::Client::new("127.0.0.1:8080")?;
        let scene = Scene::Menu(Menu::new());
        let scene = Arc::new(Mutex::new(scene));

        Ok(Client { scene, client })
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let receiver = self.client.clone();
        let scene = Arc::clone(&self.scene);
        std::thread::spawn(move || loop {
            let msg = receiver.recv().unwrap();
            match msg {
                Message::MatchCreated { player1, player2 } => {
                    println!("Match created between {} and {}", player1, player2);
                }
                Message::Sync { cards } => match &mut *scene.lock().unwrap() {
                    Scene::Game(game) => {
                        game.cards = cards;
                        println!("Synced cards: {:?}", game.cards);
                    }
                    _ => {}
                },
                _ => {}
            }
        });

        Ok(())
    }

    pub async fn step(&mut self) -> anyhow::Result<()> {
        self.process_input().await;
        self.render().await;
        Ok(())
    }

    async fn render(&mut self) {
        clear_background(RED);
        let scene = &mut *self.scene.lock().unwrap();
        scene.render(&mut self.client).await;
        // self.render_background().await;
        // self.render_deck().await;
        // self.render_player_hand().await;
        // self.render_gui().await;
    }

    async fn process_input(&mut self) {
        let current_scene = &mut *self.scene.lock().unwrap();
        let new_scene = current_scene.process_input(&mut self.client).await;
        if let Some(scene) = new_scene {
            *current_scene = scene;
        }
    }

    pub fn transition_to(&mut self, scene: Scene) {
        let mut current_scene = self.scene.lock().unwrap();
        *current_scene = scene;
    }

    // pub async fn step(&mut self) {
    //     self.process_input().await;
    //     self.update().await;
    //     self.render().await;
    //     self.check_state();
    // }
    //
    // async fn render_gui(&mut self) {
    //     let pos = vec2(SCREEN_WIDTH - 150.0, SCREEN_HEIGHT - 40.0);
    //     if root_ui().button(pos, "End Turn") {
    //         self.state.phase = Phase::EndPhase;
    //     }
    //
    //     let label_style = root_ui()
    //         .style_builder()
    //         .text_color(WHITE)
    //         .font_size(30)
    //         .build();
    //     let button_style = root_ui().style_builder().font_size(30).build();
    //
    //     let skin = Skin {
    //         label_style: label_style.clone(),
    //         button_style: button_style,
    //         tabbar_style: label_style.clone(),
    //         combobox_style: label_style.clone(),
    //         window_style: label_style.clone(),
    //         editbox_style: label_style.clone(),
    //         window_titlebar_style: label_style.clone(),
    //         scrollbar_style: label_style.clone(),
    //         scrollbar_handle_style: label_style.clone(),
    //         checkbox_style: label_style.clone(),
    //         group_style: label_style,
    //         margin: 0.0,
    //         title_height: 0.0,
    //         scroll_width: 0.0,
    //         scroll_multiplier: 0.0,
    //     };
    //     root_ui().push_skin(&skin);
    //
    //     root_ui().label(
    //         Vec2::new(10.0, SCREEN_HEIGHT - 40.0),
    //         self.state.phase.to_string().as_str(),
    //     );
    // }
    //
    // async fn render_deck(&self) {
    //     draw_texture_ex(
    //         &self.spellbook_texture,
    //         SPELLBOOK_RECT.x,
    //         SPELLBOOK_RECT.y,
    //         WHITE,
    //         DrawTextureParams {
    //             dest_size: Some(SPELLBOOK_SIZE),
    //             ..Default::default()
    //         },
    //     );
    //
    //     draw_texture_ex(
    //         &self.atlasbook_texture,
    //         ATLASBOOK_RECT.x,
    //         ATLASBOOK_RECT.y,
    //         WHITE,
    //         DrawTextureParams {
    //             dest_size: Some(ATLASBOOK_SIZE),
    //             ..Default::default()
    //         },
    //     );
    // }
    //
    // async fn render_background(&self) {
    //     draw_texture_ex(
    //         &self.realm_texture,
    //         0.0,
    //         0.0,
    //         WHITE,
    //         DrawTextureParams {
    //             dest_size: Some(vec2(SCREEN_WIDTH, SCREEN_HEIGHT)),
    //             ..Default::default()
    //         },
    //     );
    // }
    //
    // async fn render_player_hand(&self) {
    //     for card in &self.players[&self.player_id].cards_in_hand {
    //         if card.get_zone() != &CardZone::Hand {
    //             continue;
    //         }
    //
    //         let mut scale = 1.0;
    //         if card.get_is_selected() || card.get_is_hovered() {
    //             scale = 1.2;
    //         }
    //
    //         draw_texture_ex(
    //             &card.get_texture(),
    //             card.get_rect().unwrap().x,
    //             card.get_rect().unwrap().y,
    //             WHITE,
    //             DrawTextureParams {
    //                 dest_size: Some(card.get_dimensions() * Vec2::new(scale, scale)),
    //                 ..Default::default()
    //             },
    //         );
    //
    //         if card.get_is_selected() {
    //             draw_rectangle_lines(
    //                 card.get_rect().unwrap().x,
    //                 card.get_rect().unwrap().y,
    //                 card.get_dimensions().x * scale,
    //                 card.get_dimensions().y * scale,
    //                 4.0,
    //                 WHITE,
    //             );
    //         }
    //     }
    // }
    //
    // async fn update(&mut self) {
    //     let mouse_position = mouse_position();
    //     let mut hovered_card_index = None;
    //     for (idx, card) in self.players[&self.player_id]
    //         .cards_in_hand
    //         .iter()
    //         .enumerate()
    //     {
    //         if card.get_zone() != &CardZone::Hand {
    //             continue;
    //         }
    //
    //         if card.get_rect().unwrap().contains(mouse_position.into()) {
    //             hovered_card_index = Some(idx);
    //         };
    //     }
    //
    //     for card in &mut self.players.get_mut(&self.player_id).unwrap().cards_in_hand {
    //         card.set_is_hovered(false);
    //     }
    //
    //     if let Some(idx) = hovered_card_index {
    //         self.players
    //             .get_mut(&self.player_id)
    //             .unwrap()
    //             .cards_in_hand
    //             .get_mut(idx)
    //             .unwrap()
    //             .set_is_hovered(true);
    //     }
    //
    //     let player = self.players.get_mut(&self.player_id).unwrap();
    //     for card in player.cards_in_hand.iter_mut() {
    //         if card.get_zone() != &CardZone::Hand {
    //             continue;
    //         }
    //
    //         if card.get_is_hovered() && is_mouse_button_released(MouseButton::Left) {
    //             card.set_is_selected(!card.get_is_selected());
    //         };
    //     }
    // }
    //
    // async fn process_input(&mut self) {
    //     if is_mouse_button_released(MouseButton::Left) {
    //         if !self.is_current_player() {
    //             return;
    //         }
    //
    //         let mouse_position = mouse_position();
    //         if self.state.phase == Phase::WaitingForCardDrawPhase {
    //             let current_player = self.players.get_mut(&self.player_id).unwrap();
    //             let mut drew_card = false;
    //             if ATLASBOOK_RECT.contains(mouse_position.into()) {
    //                 current_player.draw_site();
    //                 drew_card = true;
    //             }
    //
    //             if SPELLBOOK_RECT.contains(mouse_position.into()) {
    //                 current_player.draw_spell();
    //                 drew_card = true;
    //             }
    //
    //             if drew_card {
    //                 self.state.next_phase();
    //             }
    //         }
    //     }
    // }
}
