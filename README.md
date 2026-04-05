# Sorcerers

Sorcerers is an unofficial platform that allows you to play **Sorcery: Contested Realm** online with friends. This project aims to provide a fun and accessible way for fans of the game to connect and play remotely.

## Disclaimer

This project is **not affiliated with, endorsed by, or associated with** the creators or publishers of Sorcery: Contested Realm. All rights to Sorcery: Contested Realm, including its name, artwork, and game mechanics, are owned by their respective copyright holders.

## Features

Sorcerers lets you play Sorcery: Contested Realm with friends in a multiplayer environment, featuring an easy-to-use interface and automated enforcement of game rules.

Sorcerers is very much a work in progress. Bugs, incomplete features, and missing functionality are to be expected. Currently, only two of the beta preconstructed decks are supported. The plan is to add more official decks and support for user-created decks in the future.

<img width="1728" height="1117" alt="image" src="https://github.com/user-attachments/assets/2ff52288-1121-4d83-ac2f-fb786b469ba8" />


## Getting Started

To install Sorcerers, download the source code and install it using Cargo:

```sh
cargo install --path .
```


Alternatively, you can run the project directly:

```sh
cargo run --release
```

Precompiled binaries will be provided in the future.

## Contributing

Contributions are welcome! This section explains how the code is organised and how to add new cards or features.

### Code Organisation

The project is split into four crates under `src/`:

| Crate | Path | Purpose |
|---|---|---|
| `lib` | `src/lib/` | Shared game logic — cards, effects, state, networking protocol |
| `client` | `src/client/` | egui GUI client |
| `server` | `src/server/` | Headless game server |
| `cli` | `src/cli/` | Command-line utilities (e.g. deck validation) |

#### `src/lib/`

The core library. Key modules:

- **`card/`** — The `Card` trait and all card implementations. `card/card.rs` defines the trait, `CardBase`, `UnitBase`, `SiteBase`, `MinionType`, `Region`, `Zone`, and shared helpers. `card/beta/` contains one file per card.
- **`effect.rs`** — The `Effect` enum, which represents every discrete game action (dealing damage, moving cards, shooting projectiles, etc.). Effects are returned by card methods and applied by the server.
- **`state.rs`** — `State` holds all in-flight game data (players, cards, mana, phase). `CardMatcher` is a composable filter for querying cards from state.
- **`game.rs`** — Input helpers (`pick_card`, `pick_zone`, `pick_direction`, `yes_or_no`) that pause execution and request input from the active player, plus data types like `PlayerId`, `Direction`, and `Thresholds`.
- **`query.rs`** — `CardQuery` and `ZoneQuery` — lazy descriptors resolved at effect-application time.
- **`networking/`** — Shared client/server message types (`ClientMessage`, `ServerMessage`).

#### `src/client/`

The GUI, built with [egui](https://github.com/emilk/egui).

- **`scene/`** — Top-level scenes: `menu`, `game`, and `deck_builder`.
- **`components/`** — Reusable egui widgets: the realm board (`realm.rs`), player hand, player status, action overlays, etc.
- **`render/`** — Draw calls for cards, triangles, and the board.
- **`texture_cache.rs`** — Lazy texture loading; card art is fetched from `curiosa.io` by edition and card name.

#### `src/server/`

Runs the authoritative game loop. Receives `ClientMessage`s, applies `Effect`s to `State`, and broadcasts `ServerMessage`s back.

### Adding a New Card

Every playable card is a Rust struct in `src/lib/card/{edition}/`. To add a new card:

1. **Create a file** `src/lib/card/{edition}/my_card.rs`.

2. **Define the struct** with a `card_base: CardBase` field and implement the `Card` trait:

    ```rust
    use crate::{
        card::{Card, CardBase, Cost, Edition, Rarity, Region, Zone},
        effect::Effect,
        game::PlayerId,
        state::State,
    };

    #[derive(Debug, Clone)]
    pub struct MyCard {
        pub card_base: CardBase,
    }

    impl MyCard {
        pub const NAME: &'static str = "My Card";

        pub fn new(owner_id: PlayerId) -> Self {
            Self {
                card_base: CardBase {
                    id: uuid::Uuid::new_v4(),
                    owner_id,
                    tapped: false,
                    zone: Zone::Spellbook,
                    cost: Cost::new(2, "AF"),  // 2 generic + 1 air + 1 fire threshold
                    region: Region::Surface,
                    rarity: Rarity::Ordinary,
                    edition: Edition::Beta,
                    controller_id: owner_id.clone(),
                    is_token: false,
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl Card for MyCard {
        fn get_name(&self) -> &str { Self::NAME }
        fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
        fn get_base(&self) -> &CardBase { &self.card_base }

        async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
            // Return a list of Effects to apply
            Ok(vec![])
        }
    }

    // Registers the card in the global registry
    #[linkme::distributed_slice(crate::card::ALL_CARDS)]
    static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
        (MyCard::NAME, |owner_id| Box::new(MyCard::new(owner_id)));
    ```

3. **Register the module** by adding `pub mod my_card;` to `src/lib/card/{edition}/mod.rs`.

The `#[linkme::distributed_slice]` macro automatically registers the card at startup — no manual registry wiring needed.

### Effects Reference

The most commonly used `Effect` variants:

| Effect | Description |
|---|---|
| `Effect::take_damage(card_id, from, damage)` | Deal damage to a card |
| `Effect::BuryCard { card_id }` | Send a unit to the cemetery |
| `Effect::BanishCard { card_id, from }` | Remove a card from the game |
| `Effect::MoveCard { .. }` | Move a card to a zone |
| `Effect::ShootProjectile { .. }` | Fire a projectile in a direction |
| `Effect::DrawSpell { player_id, count }` | Draw spell cards |
| `Effect::SummonToken { player_id, token_type, zone }` | Summon a token |
| `Effect::AddCounter { card_id, counter }` | Add a +N/+N counter |

Use `state.cards.iter()` with `CardMatcher` to query cards, and `card.get_region(state)`, `card.get_zone()`, `card.is_minion()`, etc. to inspect them.

## License

This project is licensed under a GPL-3.0 license. See [LICENSE](./LICENSE) for details.
