# Sorcerers

Sorcerers is an unofficial platform that allows you to play **Sorcery: Contested Realm** online with friends. This project aims to provide a fun and accessible way for fans of the game to connect and play remotely.

## Disclaimer

This project is **not affiliated with, endorsed by, or associated with** the creators or publishers of Sorcery: Contested Realm. All rights to Sorcery: Contested Realm, including its name, artwork, and game mechanics, are owned by their respective copyright holders.

## Features

Sorcerers lets you play Sorcery: Contested Realm with friends in a multiplayer environment, featuring a rich GUI and automated enforcement of game rules.

- **Extensive Card Support:** Over 400 cards from the Beta edition are already implemented.
- **Preconstructed Decks:** All four Beta preconstructed decks (Fire, Air, Earth, and Water) are supported out of the box.
- **Multiplayer Ready:** Connect to a headless server to play with friends.
- **Rich Visuals & Sound:** Built with `egui` for a cross-platform experience, featuring card art fetching and atmospheric sound effects.

Sorcerers is very much a work in progress. Bugs and incomplete features are to be expected.

<img width="1728" height="1117" alt="image" src="https://github.com/user-attachments/assets/2ff52288-1121-4d83-ac2f-fb786b469ba8" />

## Getting Started

### Prerequisites

You'll need the [Rust toolchain](https://rustup.rs/) installed.

### Installation

Clone the repository and build the project:

```sh
cargo build --release
```

### Running the Game

To play, you need to run both a server and a client.

1. **Start the Server:**
   ```sh
   cargo run --release --bin server
   ```

2. **Start the Client:**
   ```sh
   cargo run --release --bin client
   ```

By default, the client connects to `127.0.0.1:5000`. You can override this with the `SORCERERS_SERVER_URL` environment variable.

## Contributing

Contributions are welcome! Whether it's implementing new cards, fixing bugs, or improving the UI.

### Code Organisation

The project is a Rust workspace with three main crates:

| Crate | Path | Purpose |
|---|---|---|
| `sorcerers-core` | `src/lib/` | Shared game logic — cards, effects, state, and networking protocol. |
| `sorcerers-client` | `src/client/` | The GUI client, built with `egui`. |
| `sorcerers-server` | `src/server/` | The authoritative headless game server. |

#### Core Library (`src/lib/`)

- **`card/`**: Contains the `Card` trait and individual card implementations (organized by edition).
- **`effect.rs`**: Defines the `Effect` enum, representing all discrete game actions.
- **`state.rs`**: Holds the authoritative game state.
- **`game.rs`**: Input helpers and core game loop types.

#### Client (`src/client/`)

- **`scene/`**: Top-level game scenes (Menu, Game, Deck Builder).
- **`components/`**: Reusable UI components like the realm board, player hand, and status bars.
- **`texture_cache.rs`**: Handles asynchronous fetching of card artwork.

#### Server (`src/server/`)

- Authoritative game loop that manages connections, applies effects, and broadcasts state updates to clients.

### Adding a New Card

Adding a card is straightforward. Create a new file in `src/lib/card/beta/` and implement the `Card` trait. Cards are automatically registered using the `linkme` crate.

Refer to [AGENTS.md](./AGENTS.md) for detailed implementation guidelines and conventions.

## License

This project is licensed under a GPL-3.0 license. See [LICENSE](./LICENSE.txt) for details.
