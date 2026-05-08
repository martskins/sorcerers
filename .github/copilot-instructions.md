# Copilot Instructions — Sorcerers

This repository is a Rust monorepo for **Sorcerers**, containing core game logic, a GUI client, a headless server, and CLI tooling.

Follow these instructions when making changes.

---

# 1. Repository Overview

## Workspace Structure

This repo contains 4 crates under `src/`:

| Crate | Purpose |
|------|---------|
| `lib` | Core game logic (cards, effects, state, networking) |
| `client` | egui GUI client |
| `server` | Headless game server |
| `cli` | Command-line utilities (deck validation, tooling) |

## Important Modules

### Core Engine
- `src/lib/card/`
  - Card implementations
  - One file per card
- `src/lib/effect.rs`
  - Defines all game actions as `Effect` enum variants
- `src/lib/state.rs`
  - Defines full game state
- `src/lib/networking/`
  - Shared client/server networking types

### Client
- `src/client/bin/components/`
  - Reusable egui widgets
- `src/client/bin/texture_cache.rs`
  - Async loading of card art from curiosa.io

### Networking
- Client connects to server
- Default server address:

```text
127.0.0.1:5000
```

- Server:
  - runs game loop
  - applies effects
  - broadcasts state

---

# 2. Build / Test / Lint

## Build

Build everything:

```sh
cargo build
```

Build client:

```sh
cargo build --bin client
```

Build server:

```sh
cargo build --bin server
```

Build CLI:

```sh
cargo build --bin sorcerers-cli
```

## Run

Run client:

```sh
cargo run --bin client
```

Run server:

```sh
cargo run --bin server
```

## Test

Run all tests:

```sh
cargo test
```

Run single test:

```sh
cargo test <test_name>
```

## Format / Lint

Check formatting:

```sh
cargo fmt -- --check
```

---

# 3. Card Implementation Rules

## Definition of "Unimplemented Card"

A card is considered **unimplemented** if BOTH are true:

1. It does NOT have its own file in:

```text
src/lib/card/{edition}/
```

2. It is NOT registered via:

```rust
#[linkme::distributed_slice]
```

## IMPORTANT: Card Name Matching

When checking whether a card exists:

**Always check the `NAME` constant inside the card struct.**

Do NOT rely on filename alone.

Example:

File:

```text
angels_egg.rs
```

Card name:

```rust
const NAME: &'static str = "Angel's Egg";
```

The `NAME` constant is authoritative.

---

# 4. How To Implement A Card

When adding a card, follow this exact process.

## Step 1 — Create File

Create:

```text
src/lib/card/{edition}/{card_name}.rs
```

Filename rules:

- snake_case
- remove apostrophes
- remove special characters

Example:

```text
Angel's Egg -> angels_egg.rs
```

---

## Step 2 — Define Struct

Pattern:

```rust
pub struct MyCard {
    pub card_base: CardBase,
}
```

Implement:

```rust
impl Card for MyCard
```

---

## Step 3 — Register Card

Must register in:

### module tree

```rust
mod my_card;
```

### distributed slice

```rust
#[linkme::distributed_slice]
```

Card is not discoverable until both are done.

---

## Step 4 — Reuse Existing Mechanics

Prefer existing systems.

Before adding new logic:

Check:

```text
src/lib/effect.rs
```

for existing `Effect` variants.

Use existing effects whenever possible.

Do NOT create ad-hoc mechanics if an existing effect already models the behavior.

---

## Step 5 — Check Rules Sources

Before implementing any card, consult:

### Rulebook

```text
documents/rulebook.md
```

### Codex

```text
documents/codex.csv
```

These define game mechanics.

Treat them as source of truth.

---

## Step 6 — Beta Cards

For Beta cards, ALWAYS verify against:

```text
documents/Sorcery Contested Realm Product Tracker - Beta.csv
```

Confirm:

- cost
- thresholds
- type
- subtype
- rarity
- Curiosa slug
- card description

## MOST IMPORTANT

Use the description from the CSV exactly.

CSV text is authoritative.

---

# 5. Engine Conventions

## Effects

All gameplay actions should be represented as:

```rust
Effect
```

Server processes effects.

Do not mutate game state directly unless that is already established engine convention.

Prefer:

```rust
create Effect -> server processes -> state updates
```

over ad-hoc state mutation.

---

## UI Components

Reusable egui UI components implement:

```rust
Component
```

with:

- `update`
- `render`
- `process_input`

Follow this pattern.

---

## Texture Loading

Card art is loaded asynchronously from:

```text
curiosa.io
```

via:

```rust
TextureCache
```

Prefer existing texture cache behavior over custom image loading.

---

# 6. AI Assistant Guidance

## DO

- follow existing patterns
- reuse `Effect`s
- check rulebook before coding mechanics
- check CSV before coding Beta cards
- check `NAME` constant, not filename
- register cards properly

## DO NOT

- invent mechanics without checking existing effects
- trust filename over `NAME`
- skip distributed slice registration
- skip Beta CSV validation
- mutate state ad-hoc if an `Effect` should be used

---

# 7. Preferred Workflow For Changes

When implementing a card:

1. Find official card data  
2. Read rulebook / codex  
3. Search for similar implemented cards  
4. Reuse existing effects  
5. Implement card file  
6. Register module  
7. Register distributed slice  
8. Run:

```sh
cargo test
cargo fmt -- --check
```

9. Verify behavior

Follow this workflow unless explicitly instructed otherwise.
