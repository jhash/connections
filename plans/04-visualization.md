# Plan 04: Visualization

**Goal:** Produce structured run logs from agent solves and render them as live streaming replays in the web UI and exportable video.

**Dependencies:** Plan 03 (agent harness produces run logs). Plan 08 (web service hosts the SSE endpoint and watch page).

---

## Context

A key differentiator of this project is making agent reasoning *legible* — not just a solve rate, but a step-by-step view of what the model considered and why it guessed what it guessed. This serves both research (understanding failure modes) and outreach (showing off the project).

---

## Design Decisions

- **Run log**: newline-delimited JSON (`.ndjson`) for streaming compatibility. Each line is one turn event.
- **SSE**: server-sent events over HTTP keep the web watch page simple — HTMX `hx-ext="sse"` listens and appends turns as they arrive.
- **Video**: `asciinema` for terminal-based replay (zero dependencies, shareable as `.cast` file); headless Chromium + `ffmpeg` for web video (higher fidelity, better for sharing).
- **Comparative view**: two `<div>` panels side-by-side, each subscribed to its own SSE stream.

---

## Run Log Schema

Stored in `runs/<run_id>.ndjson`. Each line:

```json
{ "type": "start",  "run_id": "abc123", "model": "gpt-4o", "mode": "iterative", "puzzle_date": "2026-06-05", "puzzle_id": 1089, "timestamp": "2026-06-05T10:00:00Z" }
{ "type": "turn",   "turn": 1, "prompt": "...", "raw_response": "...", "parsed_guess": ["WITCH","OVEN","FOREST","COTTAGE"], "result": "correct", "category": "ASSOCIATED WITH HANSEL AND GRETEL", "one_away": false, "lives_remaining": 4, "reasoning": "These all appear in the fairy tale...", "confidence": 0.92, "latency_ms": 1240 }
{ "type": "turn",   "turn": 2, ... }
{ "type": "end",    "solved": true, "turns_taken": 4, "categories_solved": 4 }
```

`run_id` = `{date}_{model}_{mode}_{uuid8}`.

---

## Implementation Steps

### 1. Run log writer (in agent harness, Plan 03)

`src/run_log.rs`:
```rust
pub struct RunLog { file: BufWriter<File> }
impl RunLog {
    pub fn write_event(&mut self, event: &impl Serialize);
}
```

CLI harness writes events synchronously as they occur. `--output` defaults to `runs/<run_id>.ndjson`.

### 2. SSE endpoint (`connections-web`)

```
GET /watch/:run_id/stream   → text/event-stream
```

Handler:
- If run is live: tail the `.ndjson` file (inotify/kqueue), emit each new line as an SSE event
- If run is complete: read file, replay all events with simulated delay (`--speed 1.0` query param)

Each SSE event: `event: turn\ndata: <json>\n\n`.

### 3. HTMX watch page

```
GET /watch/:run_id
```

Maud template:
```html
<div id="board" hx-ext="sse" sse-connect="/watch/{run_id}/stream" sse-swap="turn">
  <!-- board updates appended here -->
</div>
```

Each `turn` event appends a rendered card group (correct → colored band, wrong → flash red).

### 4. Comparative view

```
GET /compare?a=<run_id>&b=<run_id>
```

Two side-by-side panels, each with its own SSE connection. Synchronized start via a `?sync=1` param that waits for both streams to have a `start` event before beginning.

### 5. Terminal video export

```bash
connections replay --run-id abc123 --format asciinema --output run.cast
asciinema play run.cast
```

`cmd_replay` drives a Ratatui render loop, writes frames to `asciinema` `.cast` format (JSON lines with timestamps).

### 6. Web video export

```bash
connections replay --run-id abc123 --format mp4 --output run.mp4
```

- Launches headless Chromium pointing at `/watch/:run_id?autoplay=1`
- Records with Puppeteer's `page.screencast()` or `ffmpeg` pulling from virtual display
- Requires `chromium` + `ffmpeg` in PATH; document in README

---

## Verification

```bash
# Generate a test run
connections solve --model gpt-4o --mode iterative --date 2024-01-15

# Verify log
ls runs/
cat runs/*.ndjson | jq -s 'length'   # expect: N turn events

# SSE stream (offline replay)
curl -N "http://localhost:3062/watch/<run_id>/stream"
# expect: stream of SSE events with turn data

# Watch page
open http://localhost:3062/watch/<run_id>
# verify board updates as events arrive

# Terminal replay
connections replay --run-id <run_id> --format asciinema --output /tmp/test.cast
asciinema play /tmp/test.cast
```
