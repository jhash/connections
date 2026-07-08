# Plan 02: Interfaces (TUI + Web UI)

**Goal:** Deliver a playable terminal UI and a web UI that share the same core game logic, serving both human players and as the live showcase surface for agent visualization.

**Dependencies:** Plan 01 (annotated archive) for metadata display. Plan 04 (visualization) for live agent watch. Plan 08 (web service) is the deployment target for the web UI — this plan covers the game layer only.

---

## Context

The game has simple, well-defined rules: 16 words, 4 groups of 4, 4 lives, difficulty bands (yellow/green/blue/purple). The challenge is sharing logic between TUI and web without duplication. The solution is a `connections-core` library crate that both the TUI binary and the Axum server depend on.

---

## Design Decisions

- **Shared core**: `crates/connections-core/` — `GameState`, validation, archive I/O. Both TUI and web are thin consumers.
- **TUI**: [Ratatui](https://ratatui.rs/) — mature, actively maintained, good grid layout primitives.
- **Web UI**: Axum + [Maud](https://maud.lambda.xyz/) (typed HTML macros) + HTMX. No JS framework. Guess submission is a `POST /play/guesses` that returns an HTML fragment; HTMX swaps it in.
- **Workspace layout**: convert repo to a Cargo workspace to cleanly split the crates.

---

## Implementation Steps

### 1. Cargo workspace refactor
Convert to workspace in root `Cargo.toml`:
```toml
[workspace]
members = ["crates/connections-core", "crates/connections-cli", "crates/connections-web"]
```
Move existing `src/main.rs` to `crates/connections-cli/src/main.rs`. Archive logic stays in CLI.

### 2. `connections-core` crate

```
crates/connections-core/src/
  lib.rs
  game.rs      # GameState, GuessResult, DifficultyBand
  puzzle.rs    # Puzzle, Category, Card (moved from CLI)
  archive.rs   # load_archive, load_puzzle_by_date
```

Key types:
```rust
pub struct GameState {
    pub puzzle: Puzzle,
    pub lives: u8,
    pub solved: Vec<usize>,       // indices of solved categories
    pub guesses: Vec<[usize; 4]>, // card index tuples
}

pub enum GuessResult {
    Correct(usize),   // category index
    OneAway,
    Wrong,
    AlreadySolved,
}

impl GameState {
    pub fn new(puzzle: Puzzle, lives: u8) -> Self;
    pub fn submit_guess(&mut self, cards: [usize; 4]) -> GuessResult;
    pub fn is_solved(&self) -> bool;
    pub fn is_game_over(&self) -> bool;
}
```

### 3. TUI (`connections-cli` — new `play` subcommand)

Add `Play { date: Option<String> }` variant to `Command`. Delegate to `cmd_play(date)`.

Layout (Ratatui):
```
┌─────────────────────────────────┐
│  NYT Connections #1089 — 2026-06-05   ❤❤❤❤ │
├────────┬────────┬────────┬───────┤
│ WORD1  │ WORD2  │ WORD3  │ WORD4 │   ← row 1
│ WORD5  │ WORD6  │ WORD7  │ WORD8 │   ← row 2
│ WORD9  │ WORD10 │ WORD11 │ WORD12│   ← row 3
│ WORD13 │ WORD14 │ WORD15 │ WORD16│   ← row 4
├─────────────────────────────────┤
│ Selected: WORD1, WORD3          │
│ [Enter] Guess  [Esc] Clear  [Q] Quit │
└─────────────────────────────────┘
```

- Arrow keys / hjkl to move cursor; Space to toggle selection; Enter to submit 4-card guess
- Solved categories collapse into a colored band at top with title revealed
- Color mapping: yellow=0, green=1, blue=2, purple=3 (by category index, matching NYT difficulty order)

**Replay mode** (`connections play --replay 2026-06-05`):
- Loads solved puzzle from archive; steps through guess sequence on keypress

### 4. Web UI (`connections-web` crate)

Routes handled here (game logic only; full route list in Plan 08):

```
GET  /play                    → render game board (date=today or ?date=YYYY-MM-DD)
POST /play/guesses              → submit guess, return HTMX board fragment
GET  /play/random             → redirect to /play?date=<random archive date>
```

Maud template for board:
```rust
html! {
    div #game-board hx-target="this" hx-swap="outerHTML" {
        // 16 card buttons, each with hx-vals for card index
        // solved bands rendered above
        // lives display
    }
}
```

POST handler receives `[usize; 4]` from form body, calls `game_state.submit_guess(...)`, returns updated board HTML fragment. Session state stored server-side (in-memory HashMap keyed by session cookie) or in URL-encoded state for simplicity.

---

## Verification

```bash
# TUI
cargo run -p connections-cli -- play
# manually play a puzzle; verify 4-life counter, color bands on solve, game-over screen

cargo run -p connections-cli -- play --replay 2026-06-05
# verify step-through replay

# Web
cargo run -p connections-web
# open http://localhost:3062/play
# submit guesses; verify HTMX swaps board; verify lives decrement; verify solved band appears
```
