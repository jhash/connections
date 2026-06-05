# connections

Rust CLI to fetch and archive [NYT Connections](https://www.nytimes.com/games/connections) puzzle data.

## Build

```bash
cargo build --release
```

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
