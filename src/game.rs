use std::collections::HashMap;

use crate::card::CardZone;
use crate::deck::Deck;
use crate::player::Player;
use crate::window::*;
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Skin};

#[derive(PartialEq, Debug)]
pub enum Phase {
    None,
    TurnStartPhase,
    WaitingForCardDrawPhase,
    WaitingForCellSelectionPhase,
    MainPhase,
    EndPhase,
}

impl ToString for Phase {
    fn to_string(&self) -> String {
        match self {
            Phase::None => "None".to_string(),
            Phase::TurnStartPhase => "Turn Start Phase".to_string(),
            Phase::WaitingForCardDrawPhase => "Waiting For Card Draw Phase".to_string(),
            Phase::WaitingForCellSelectionPhase => "Waiting For Cell Selection Phase".to_string(),
            Phase::MainPhase => "Main Phase".to_string(),
            Phase::EndPhase => "End Phase".to_string(),
        }
    }
}

impl Phase {
    fn next(&self) -> Phase {
        match self {
            Phase::None => Phase::TurnStartPhase,
            Phase::TurnStartPhase => Phase::WaitingForCardDrawPhase,
            Phase::WaitingForCardDrawPhase => Phase::MainPhase,
            Phase::MainPhase => Phase::EndPhase,
            Phase::EndPhase => Phase::TurnStartPhase,
            _ => Phase::None,
        }
    }
}

pub struct State {
    phase: Phase,
    turn_count: u32,
    current_player: uuid::Uuid,
    next_player: uuid::Uuid,
    selected_cards: Vec<uuid::Uuid>,
}

impl State {
    pub fn zero() -> Self {
        State {
            phase: Phase::None,
            turn_count: 0,
            current_player: uuid::Uuid::nil(),
            next_player: uuid::Uuid::nil(),
            selected_cards: vec![],
        }
    }

    fn next_phase(&mut self) {
        self.phase = self.phase.next();
    }
}

pub struct Game {
    players: HashMap<uuid::Uuid, Player>,
    player_id: uuid::Uuid,
    spellbook_texture: Texture2D,
    atlasbook_texture: Texture2D,
    realm_texture: Texture2D,
    state: State,
}

impl Game {
    pub async fn setup() -> Self {
        let spellbook_texture: Texture2D = load_texture("assets/images/cards/Spell Back.webp")
            .await
            .unwrap();
        let atlasbook_texture: Texture2D = load_texture("assets/images/cards/Site Back.webp")
            .await
            .unwrap();
        let realm_texture: Texture2D = load_texture("assets/images/Realm.jpg").await.unwrap();

        let deck = Deck::test_deck().await;
        let player_one = Player::new("Player 1".to_string(), Deck::from(&deck));
        let player_id = player_one.id;
        let player_two = Player::new("Player 2".to_string(), deck);

        Game {
            players: HashMap::from([(player_one.id, player_one), (player_two.id, player_two)]),
            player_id,
            spellbook_texture,
            atlasbook_texture,
            realm_texture,
            state: State::zero(),
        }
    }

    pub fn start(&mut self) {
        for (_, player) in &mut self.players {
            for _ in 0..2 {
                player.draw_site();
                player.draw_spell();
            }
        }
    }

    pub async fn step(&mut self) {
        self.process_input().await;
        self.update().await;
        self.render().await;
        self.check_state();
    }

    pub fn check_state(&mut self) {
        let player_ids: Vec<uuid::Uuid> = self.players.keys().cloned().collect();
        match self.state.phase {
            Phase::None => {
                println!("Starting turn for player {}", player_ids[0]);
                self.state.current_player = *player_ids.get(0).unwrap();
                self.state.next_player = *player_ids.get(1).unwrap();
                self.state.turn_count += 1;
                self.state.next_phase();
            }
            Phase::TurnStartPhase => {
                // Handle turn start logic
                self.state.next_phase();
            }
            Phase::MainPhase => {
                // Handle main phase logic
            }
            Phase::EndPhase => {
                // Handle end phase logic
                let current_player_id = self.state.current_player;
                let next_player_id = self.state.next_player;
                self.state.current_player = next_player_id;
                self.state.next_player = current_player_id;
                self.state.next_phase();
                println!("Starting turn for player {}", self.state.current_player);
            }
            _ => {}
        }
    }

    fn is_current_player(&self) -> bool {
        println!(
            "Checking player {} vs {}",
            self.player_id, self.state.current_player
        );
        self.player_id == self.state.current_player
    }

    async fn update(&mut self) {
        let mouse_position = mouse_position();
        let mut hovered_card_index = None;
        for (idx, card) in self.players[&self.player_id]
            .cards_in_hand
            .iter()
            .enumerate()
        {
            if card.get_zone() != &CardZone::Hand {
                continue;
            }

            if card.get_rect().unwrap().contains(mouse_position.into()) {
                hovered_card_index = Some(idx);
            };
        }

        for card in &mut self.players.get_mut(&self.player_id).unwrap().cards_in_hand {
            card.set_is_hovered(false);
        }

        if let Some(idx) = hovered_card_index {
            self.players
                .get_mut(&self.player_id)
                .unwrap()
                .cards_in_hand
                .get_mut(idx)
                .unwrap()
                .set_is_hovered(true);
        }

        let player = self.players.get_mut(&self.player_id).unwrap();
        for card in player.cards_in_hand.iter_mut() {
            if card.get_zone() != &CardZone::Hand {
                continue;
            }

            if card.get_is_hovered() && is_mouse_button_released(MouseButton::Left) {
                card.set_is_selected(!card.get_is_selected());
            };
        }
    }

    async fn process_input(&mut self) {
        if is_mouse_button_released(MouseButton::Left) {
            if !self.is_current_player() {
                return;
            }

            let mouse_position = mouse_position();
            if self.state.phase == Phase::WaitingForCardDrawPhase {
                let current_player = self.players.get_mut(&self.player_id).unwrap();
                let mut drew_card = false;
                if ATLASBOOK_RECT.contains(mouse_position.into()) {
                    current_player.draw_site();
                    drew_card = true;
                }

                if SPELLBOOK_RECT.contains(mouse_position.into()) {
                    current_player.draw_spell();
                    drew_card = true;
                }

                if drew_card {
                    self.state.next_phase();
                }
            }

            if self.state.phase == Phase::MainPhase {}
        }
    }

    async fn render(&mut self) {
        clear_background(RED);
        self.render_background().await;
        self.render_deck().await;
        self.render_player_hand().await;
        self.render_gui().await;
    }

    async fn render_gui(&mut self) {
        let pos = vec2(SCREEN_WIDTH - 150.0, SCREEN_HEIGHT - 40.0);
        if root_ui().button(pos, "End Turn") {
            self.state.phase = Phase::EndPhase;
        }

        let label_style = root_ui()
            .style_builder()
            .text_color(WHITE)
            .font_size(30)
            .build();
        let button_style = root_ui().style_builder().font_size(30).build();

        let skin = Skin {
            label_style: label_style.clone(),
            button_style: button_style,
            tabbar_style: label_style.clone(),
            combobox_style: label_style.clone(),
            window_style: label_style.clone(),
            editbox_style: label_style.clone(),
            window_titlebar_style: label_style.clone(),
            scrollbar_style: label_style.clone(),
            scrollbar_handle_style: label_style.clone(),
            checkbox_style: label_style.clone(),
            group_style: label_style,
            margin: 0.0,
            title_height: 0.0,
            scroll_width: 0.0,
            scroll_multiplier: 0.0,
        };
        root_ui().push_skin(&skin);

        root_ui().label(
            Vec2::new(10.0, SCREEN_HEIGHT - 40.0),
            self.state.phase.to_string().as_str(),
        );
    }

    async fn render_deck(&self) {
        draw_texture_ex(
            &self.spellbook_texture,
            SPELLBOOK_RECT.x,
            SPELLBOOK_RECT.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(SPELLBOOK_SIZE),
                ..Default::default()
            },
        );

        draw_texture_ex(
            &self.atlasbook_texture,
            ATLASBOOK_RECT.x,
            ATLASBOOK_RECT.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(ATLASBOOK_SIZE),
                ..Default::default()
            },
        );
    }

    async fn render_background(&self) {
        draw_texture_ex(
            &self.realm_texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(SCREEN_WIDTH, SCREEN_HEIGHT)),
                ..Default::default()
            },
        );
    }

    async fn render_player_hand(&self) {
        for card in &self.players[&self.player_id].cards_in_hand {
            if card.get_zone() != &CardZone::Hand {
                continue;
            }

            let mut scale = 1.0;
            if card.get_is_selected() || card.get_is_hovered() {
                scale = 1.2;
            }

            draw_texture_ex(
                &card.get_texture(),
                card.get_rect().unwrap().x,
                card.get_rect().unwrap().y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(card.get_dimensions() * Vec2::new(scale, scale)),
                    ..Default::default()
                },
            );

            if card.get_is_selected() {
                draw_rectangle_lines(
                    card.get_rect().unwrap().x,
                    card.get_rect().unwrap().y,
                    card.get_dimensions().x * scale,
                    card.get_dimensions().y * scale,
                    4.0,
                    WHITE,
                );
            }
        }
    }
}
