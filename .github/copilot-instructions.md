# Copilot Instructions for Sorcerers

## Build, Test, and Lint Commands

- **Build all binaries:**
  ```sh
  cargo build --release
  ```
- **Build client only:**
  ```sh
  cargo build --bin client
  ```
- **Build server only:**
  ```sh
  cargo build --bin server
  ```
- **Build CLI only:**
  ```sh
  cargo build --bin sorcerers-cli
  ```
- **Run client:**
  ```sh
  cargo run --bin client --release
  ```
- **Run server:**
  ```sh
  cargo run --bin server --release
  ```
- **Run a single test:**
  ```sh
  cargo test <test_name>
  ```
- **Run all tests:**
  ```sh
  cargo test
  ```
- **Lint (format check):**
  ```sh
  cargo fmt -- --check
  ```

## High-Level Architecture

- **Monorepo with 4 crates in `src/`:**
  - `lib`: Core game logic (cards, effects, state, networking)
  - `client`: egui-based GUI client
  - `server`: Headless game server
  - `cli`: Command-line utilities (e.g., deck validation)
- **Key modules:**
  - `src/lib/card/`: Card trait and implementations (one file per card)
  - `src/lib/effect.rs`: Effect enum for all game actions
  - `src/lib/state.rs`: State struct for all game data
  - `src/lib/networking/`: Shared client/server message types
  - `src/client/bin/components/`: Reusable egui widgets
  - `src/client/bin/texture_cache.rs`: Async texture loading for card art
- **Client/Server:**
  - Client connects to server (default: `127.0.0.1:5000`).
  - Server runs game loop, applies effects, and broadcasts state.

## Key Conventions

- An unimplemented card is a card that does not have its own file in `src/lib/card/{edition}/` and is not registered with `#[linkme::distributed_slice]`.
- When checking for unimplemented cards, make sure you check the name of the card in the NAME constant in the card's struct, not just the file name, as some cards have slightly different names in the file (e.g., "Angel's Egg" is "angels_egg.rs").
- **Implementing a Card:**
  1. Create a file in `src/lib/card/{edition}/` (e.g., `my_card.rs`). Make sure to remove apostrophes and special characters from the file name, and use snake_case.
  2. Define a struct with a `card_base: CardBase` field and implement the `Card` trait.
  3. Register the module in `mod.rs` and with `#[linkme::distributed_slice]`.
  4. Try and use existing card mechanics instead of implement ad-hoc effects. Check `src/lib/effect.rs` for existing effects and `documents/rulebook.md` for game mechanics.
  5. Check `documents/Sorcery Contested Realm Product Tracker - Beta.csv` in the repo root before coding a Beta card. Use the row for the card name to confirm cost, thresholds, type, subtype, rarity, and Curiosa slug. Also, and most important of all, make sure the card description is the one in the CSV.
  6. Check `documents/rulebook.md` in the repo root for rules of the game and card mechanics before coding any card.
- **Effects:**
  - All game actions are represented as `Effect` variants and processed by the server.
- **Async Texture Loading:**
  - Card art is loaded asynchronously from `curiosa.io` using `TextureCache`.
- **egui UI:**
  - All UI components implement the `Component` trait with `update`, `render`, and `process_input` methods.
- **Networking:**
  - Uses `tokio` and message types in `src/lib/networking/message.rs`.
- **Testing:**
  - Use standard Rust `cargo test` for all crates.

---

This file summarizes build/test commands, architecture, and key conventions for Copilot and other AI assistants. If you want to adjust coverage or add more areas, let me know!
