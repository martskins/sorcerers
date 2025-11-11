pub enum Phase {
    None,
    TurnStartPhase,
    WaitingForCardDrawPhase,
    WaitingForCellSelectionPhase,
    MainPhase,
    EndPhase,
}

pub struct State {
    pub phase: Phase,
    pub turn_count: u32,
    pub current_player: uuid::Uuid,
    pub next_player: uuid::Uuid,
    pub selected_cards: Vec<String>,
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
}

pub struct Game {
    pub players: Vec<uuid::Uuid>,
    pub state: State,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid) -> Self {
        Game {
            players: vec![player1, player2],
            state: State::zero(),
        }
    }
}

// use std::collections::HashMap;
//
// use crate::deck::Deck;
// use crate::player::Player;
// use macroquad::prelude::*;
//
// #[derive(PartialEq, Debug)]
// pub enum Phase {
//     None,
//     TurnStartPhase,
//     WaitingForCardDrawPhase,
//     WaitingForCellSelectionPhase,
//     MainPhase,
//     EndPhase,
// }
//
// impl ToString for Phase {
//     fn to_string(&self) -> String {
//         match self {
//             Phase::None => "None".to_string(),
//             Phase::TurnStartPhase => "Turn Start Phase".to_string(),
//             Phase::WaitingForCardDrawPhase => "Waiting For Card Draw Phase".to_string(),
//             Phase::WaitingForCellSelectionPhase => "Waiting For Cell Selection Phase".to_string(),
//             Phase::MainPhase => "Main Phase".to_string(),
//             Phase::EndPhase => "End Phase".to_string(),
//         }
//     }
// }
//
// impl Phase {
//     fn next(&self) -> Phase {
//         match self {
//             Phase::None => Phase::TurnStartPhase,
//             Phase::TurnStartPhase => Phase::WaitingForCardDrawPhase,
//             Phase::WaitingForCardDrawPhase => Phase::MainPhase,
//             Phase::MainPhase => Phase::EndPhase,
//             Phase::EndPhase => Phase::TurnStartPhase,
//             _ => Phase::None,
//         }
//     }
// }
//
// pub struct State {
//     phase: Phase,
//     turn_count: u32,
//     current_player: uuid::Uuid,
//     next_player: uuid::Uuid,
//     selected_cards: Vec<uuid::Uuid>,
// }
//
// impl State {
//     pub fn zero() -> Self {
//         State {
//             phase: Phase::None,
//             turn_count: 0,
//             current_player: uuid::Uuid::nil(),
//             next_player: uuid::Uuid::nil(),
//             selected_cards: vec![],
//         }
//     }
//
//     fn next_phase(&mut self) {
//         self.phase = self.phase.next();
//     }
// }
//
// pub struct Game {
//     players: HashMap<uuid::Uuid, Player>,
//     player_id: uuid::Uuid,
//     spellbook_texture: Texture2D,
//     atlasbook_texture: Texture2D,
//     realm_texture: Texture2D,
//     state: State,
// }
//
// impl Game {
//     pub async fn setup() -> Self {
//         let spellbook_texture: Texture2D = load_texture("assets/images/cards/Spell Back.webp")
//             .await
//             .unwrap();
//         let atlasbook_texture: Texture2D = load_texture("assets/images/cards/Site Back.webp")
//             .await
//             .unwrap();
//         let realm_texture: Texture2D = load_texture("assets/images/Realm.jpg").await.unwrap();
//
//         let deck = Deck::test_deck().await;
//         let player_one = Player::new("Player 1".to_string(), Deck::from(&deck));
//         let player_id = player_one.id;
//         let player_two = Player::new("Player 2".to_string(), deck);
//
//         Game {
//             players: HashMap::from([(player_one.id, player_one), (player_two.id, player_two)]),
//             player_id,
//             spellbook_texture,
//             atlasbook_texture,
//             realm_texture,
//             state: State::zero(),
//         }
//     }
//
//     pub fn start(&mut self) {
//         for (_, player) in &mut self.players {
//             for _ in 0..2 {
//                 player.draw_site();
//                 player.draw_spell();
//             }
//         }
//     }
//
//     pub fn check_state(&mut self) {
//         let player_ids: Vec<uuid::Uuid> = self.players.keys().cloned().collect();
//         match self.state.phase {
//             Phase::None => {
//                 println!("Starting turn for player {}", player_ids[0]);
//                 self.state.current_player = *player_ids.get(0).unwrap();
//                 self.state.next_player = *player_ids.get(1).unwrap();
//                 self.state.turn_count += 1;
//                 self.state.next_phase();
//             }
//             Phase::TurnStartPhase => {
//                 // Handle turn start logic
//                 self.state.next_phase();
//             }
//             Phase::MainPhase => {
//                 // Handle main phase logic
//             }
//             Phase::EndPhase => {
//                 // Handle end phase logic
//                 let current_player_id = self.state.current_player;
//                 let next_player_id = self.state.next_player;
//                 self.state.current_player = next_player_id;
//                 self.state.next_player = current_player_id;
//                 self.state.next_phase();
//                 println!("Starting turn for player {}", self.state.current_player);
//             }
//             _ => {}
//         }
//     }
//
//     fn is_current_player(&self) -> bool {
//         println!(
//             "Checking player {} vs {}",
//             self.player_id, self.state.current_player
//         );
//         self.player_id == self.state.current_player
//     }
// }
