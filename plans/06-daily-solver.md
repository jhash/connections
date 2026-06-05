# Plan 06: Daily Solver Service

**Goal:** Extend the existing daily archive job to automatically run configured models against each new puzzle, record results, and publish to the leaderboard — zero manual intervention.

**Dependencies:** Plan 03 (agent harness `solve`). Plan 05 (eval leaderboard). Existing `scripts/run-daily.sh` and `scripts/install-cron.sh`.

---

## Context

The daily archive job already runs via launchd/systemd/Docker and commits `archive.json`. Extending it to run model evals requires:
1. A model registry so new models can be added without code changes
2. The solve loop integrated into `run-daily.sh` after archive update
3. Result commits + optional HTTP publish

---

## Design Decisions

- **Model registry**: `models.toml` at repo root — TOML array of model configs. Not `models.json` because TOML supports comments (useful for documenting why a model is included).
- **Local-first**: Ollama endpoints are first-class; remote API keys optional. If a model endpoint is unreachable, skip and log — don't fail the job.
- **Publish strategy**: commit `leaderboard.json` to repo (always); optionally POST to web service if `WEB_SERVICE_URL` is set in environment.
- **Rate limiting**: configurable delay between model calls; default 1s.

---

## `models.toml` Schema

```toml
[[models]]
id = "gpt-4o"
type = "openai"              # openai | anthropic | ollama
endpoint = "https://api.openai.com/v1"
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"
eval_modes = ["words-only", "iterative"]

[[models]]
id = "claude-sonnet-4-6"
type = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
eval_modes = ["iterative"]

[[models]]
id = "llama3.2-3b-local"
type = "ollama"
endpoint = "http://localhost:11434"
model = "llama3.2:3b"
eval_modes = ["words-only", "iterative"]
# local model; no api_key needed
```

---

## Implementation Steps

### 1. Model registry parser

`src/registry.rs`:
```rust
pub struct ModelConfig {
    pub id: String,
    pub backend: BackendKind,
    pub eval_modes: Vec<EvalMode>,
}

pub fn load_registry(path: &Path) -> Vec<ModelConfig>;
```

Add `toml` dependency to `Cargo.toml`.

### 2. `connections solve-daily` subcommand

```
connections solve-daily \
  --date today \
  --registry models.toml \
  --output eval_results/ \
  --leaderboard leaderboard.json
```

- Loads registry
- For each model × mode pair: calls `run_puzzle(puzzle, model, mode)`
- Writes result to `eval_results/`
- Re-generates `leaderboard.json`
- Skips if `eval_results/YYYY-MM-DD_{model}_{mode}.json` already exists (idempotent)

### 3. Extend `run-daily.sh`

After the existing archive + git commit block, add:

```bash
# Run models against today's puzzle
"$BINARY" solve-daily --date today --registry "$PROJECT_DIR/models.toml" \
  --output "$PROJECT_DIR/eval_results/" \
  --leaderboard "$PROJECT_DIR/leaderboard.json"

# Commit leaderboard and new eval results if changed
if ! git diff --quiet leaderboard.json eval_results/; then
  git add leaderboard.json eval_results/
  git commit -m "chore: eval results $today"
  git push
fi

# Optionally publish to web service
if [[ -n "${WEB_SERVICE_URL:-}" ]]; then
  curl -s -X POST "$WEB_SERVICE_URL/api/results" \
    -H "Content-Type: application/json" \
    -d @leaderboard.json
fi
```

### 4. Docker extension

Add `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` as optional env vars in `docker run` docs. Mount `models.toml` or bake into image.

### 5. Failure handling

- Unreachable endpoint (connection refused / timeout): log `WARN: model {id} unreachable, skipping` to stderr, continue
- API error (rate limit, auth): log error with HTTP status, skip model for today
- Parse failure (model returned unparseable guess): record as `{ full_solve: false, categories_solved: 0, error: "parse_failure" }`
- Daily job never fails hard due to model errors — archive update is the primary obligation

---

## Verification

```bash
# Dry run with one local model
connections solve-daily --date 2026-06-05 --registry models.toml --output /tmp/eval_test/
ls /tmp/eval_test/   # expect one file per model×mode

# Leaderboard update
cat leaderboard.json | jq '.generated_at'   # should be today

# Idempotency
connections solve-daily --date 2026-06-05 --registry models.toml --output /tmp/eval_test/
# expect: "skipping (already exists)" for all models

# run-daily.sh end-to-end
bash scripts/run-daily.sh
git log --oneline -3   # expect: archive commit + eval commit
```
