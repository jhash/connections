# Plan 08: Live Web Service

**Goal:** Ship a single Axum application that serves the human-playable game, leaderboard, agent replays, archive browser, and community author profiles — all in plain Rust with server-rendered HTML, no JS framework.

**Dependencies:** Plan 02 (game logic via `connections-core`). Plan 04 (run log SSE). Plan 05 (leaderboard data). Plan 01 (annotated archive for taxonomy display).

---

## Context

The web service is the public face of the project. It should feel fast and minimal. HTMX handles interactivity (guess submission, live replay streaming, filter updates) without a JS build step. Maud provides typed, compile-checked HTML templates. SVG charts are rendered server-side from Rust — no charting library dependency at runtime.

Stack:
- **Axum** — HTTP framework
- **Maud** — typed HTML macros (`html! { ... }`)
- **HTMX** — declarative server-driven interactivity (CDN-loaded, one `<script>` tag)
- **Tower** — middleware (compression, static files, session)
- **SQLite via rusqlite** — optional: for session state if cookie-encoded state becomes unwieldy

---

## Routes

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Landing page — project intro, latest leaderboard snapshot, links |
| GET | `/play` | Human game — today's puzzle by default |
| GET | `/play?date=YYYY-MM-DD` | Specific puzzle |
| POST | `/play/guesses` | Submit guess → returns HTMX board fragment |
| GET | `/agents` | Leaderboard — full model comparison table + charts |
| GET | `/agents/:model` | Per-model detail — taxonomy breakdown, history |
| GET | `/watch/:run_id` | Agent replay page (live or recorded) |
| GET | `/watch/:run_id/stream` | SSE event stream for replay |
| GET | `/compare?a=:run_id&b=:run_id` | Side-by-side replay |
| GET | `/archive` | Browsable puzzle list with filters |
| GET | `/archive/:date` | Single puzzle detail — solution + taxonomy labels |
| GET | `/community/:username` | Author profile + game list |
| GET | `/api/leaderboard` | JSON leaderboard (for external consumption) |

---

## Implementation Steps

### 1. Crate setup

`crates/connections-web/`:
```
src/
  main.rs         # Axum router, server setup
  routes/
    play.rs
    agents.rs
    watch.rs
    archive.rs
    community.rs
    api.rs
  templates/
    layout.rs     # base HTML shell
    board.rs      # game board fragment
    leaderboard.rs
    charts.rs     # SVG chart generators
  state.rs        # AppState (archive, leaderboard, active sessions)
```

`AppState` holds:
```rust
pub struct AppState {
    pub archive: Arc<Vec<Puzzle>>,           // loaded at startup
    pub leaderboard: Arc<RwLock<Leaderboard>>,
    pub sessions: Arc<DashMap<String, GameState>>,
    pub run_index: Arc<RunIndex>,            // maps run_id → .ndjson path
}
```

### 2. Game session handling

Session ID in a cookie. On `GET /play`: if no session, create new `GameState` for the requested date, store in `sessions` map. On `POST /play/guesses`: look up session, call `game_state.submit_guess()`, return Maud-rendered board fragment.

For simplicity, sessions are in-memory with a 24h TTL. Acceptable tradeoff: users lose game state on server restart.

### 3. SVG chart generators

`src/templates/charts.rs` — pure functions, no external dependency:

```rust
pub fn line_chart(series: &[(String, Vec<(f64, f64)>)], width: u32, height: u32) -> Markup;
pub fn heatmap(cells: &[HeatmapCell], width: u32, height: u32) -> Markup;
pub fn bar_chart(bars: &[(String, f64)], width: u32, height: u32) -> Markup;
```

Charts used:
- `/agents` page: bar chart of full-solve rates by model; heatmap of model × taxonomy accuracy
- `/agents/:model`: line chart of solve rate over time (30-day rolling average)
- `/archive/:date`: small bar showing category difficulty (attempted plays vs. puzzle average)

### 4. `/archive` with filters

HTMX-powered filter bar: filter by taxonomy class, Wyna pattern, date range, image/text. Each filter change sends `GET /archive?taxonomy=Encyclopedic&...` with `hx-push-url="true"` — bookmarkable URLs.

Results rendered as a paginated table (50 per page). Each row links to `/archive/:date`.

### 5. `/community/:username`

- Load `<username>.json`
- Show game list sorted by `created_at` desc
- Style analysis section (once categories are decryptable per Plan 01): pattern distribution bar chart vs. NYT baseline
- Link to each game on connectionsplus.io

### 6. Static assets

Single CSS file (`static/style.css`) — minimal, no framework. Served via `tower-http` `ServeDir`. HTMX loaded from CDN: `<script src="https://unpkg.com/htmx.org@2"></script>`.

### 7. Deployment

Binary is fully self-contained (no Node, no webpack). Deploy options:
- `fly.io` — single `fly.toml`, `Dockerfile` already exists
- `shuttle.rs` — Rust-native PaaS
- VPS via Docker Compose

Add `GET /health` → `200 OK` for load balancer checks.

---

## Verification

```bash
cargo run -p connections-web
# open http://localhost:3062

# Play a game
open http://localhost:3062/play
# submit 4 guesses; verify HTMX swaps board; lives decrement; solved band appears on correct guess

# Leaderboard
open http://localhost:3062/agents
# verify model table renders; SVG charts visible

# Agent watch (requires a run log)
connections solve --model gpt-4o --date 2024-01-15
open "http://localhost:3062/watch/$(ls runs/ | head -1 | sed 's/.ndjson//')"
# verify board replays step by step

# Archive browser
open "http://localhost:3062/archive?taxonomy=Encyclopedic"
# verify filter works; rows link to puzzle detail

# Community profile
open http://localhost:3062/community/chloetron
# verify 49 games listed

# API
curl http://localhost:3062/api/leaderboard | jq '.entries | length'
```
