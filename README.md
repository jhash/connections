# connections

Research platform for studying and solving the [NYT Connections](https://www.nytimes.com/games/connections) word game with language models. The immediate goal is a complete puzzle archive and tooling for reproducible evaluation; the longer-term goal is fine-tuned models — ideally small, local ones — that can solve puzzles reliably, benchmarked against the full NYT archive and community puzzles from [connectionsplus.io](https://connectionsplus.io). See [TODO.md](TODO.md) for the full roadmap covering agent harness, evaluation framework, TUI/web UI, daily solver service, and fine-tuning pipeline. See [PAPERS.md](PAPERS.md) for an overview of prior work.

## Build

```bash
cargo build --release
```

The repo is a [Cargo workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html). Source lives in `crates/`:
- `crates/connections-cli/` — the CLI binary (`connections`)
- `crates/connections-core/` — shared game logic (grows as features are added)

`cargo build --release` from the repo root builds everything. The binary is always at `target/release/connections`.

## CLI

### `words` — all 16 words sorted by board position

```bash
./target/release/connections words           # today
./target/release/connections words 2026-04-01
```

### `json` — raw JSON from the NYT API

```bash
./target/release/connections json
./target/release/connections json 2026-04-01
```

### `archive` — incremental fetch, today → `--since`, skips cached dates

```bash
./target/release/connections archive                              # appends to archive.json
./target/release/connections archive --output data.json --since 2024-01-01
```

Re-run anytime — already-fetched dates are skipped. Image-based puzzles (April Fools, Halloween, etc.) are stored with `image_url` and `image_alt_text` in place of `content`.

---

## Daily auto-archive

Fetches new puzzles once per day, commits and pushes `archive.json` if it changed.

### macOS or Linux VM

```bash
bash scripts/install-cron.sh
```

- **macOS** — installs a launchd agent (`~/Library/LaunchAgents/com.jhash.connections.plist`) that fires every 2 hours
- **Linux** — installs a systemd user timer (`~/.config/systemd/user/connections-archive.timer`) that fires every 2 hours

Both use a `.last-run` stamp file so work runs at most once per calendar day. Failed runs retry on the next 2-hour tick.

Logs → `connections.log` in project root.

### Docker (cloud VM or scheduled container)

Build the image once (binary compiled inside):

```bash
docker build -t connections-archive .
```

Run daily via your cloud scheduler or a host cron job:

```bash
docker run --rm \
  -v /path/to/repo:/repo \
  -v ~/.ssh:/root/.ssh:ro \
  connections-archive
```

The container expects:
- `/repo` — the cloned git repo (persists `archive.json` and `.last-run` between runs)
- `/root/.ssh` — SSH keys with push access to the remote

The same once-per-day gate applies inside the container.

---

## Data source

```
GET https://www.nytimes.com/svc/connections/v2/YYYY-MM-DD.json
```

Publicly accessible, no authentication required.
