# Plan 05: Evaluation Framework

**Goal:** Implement a reproducible evaluation pipeline that runs any model against the full archive, computes standardized metrics, stratifies by knowledge taxonomy and Wyna pattern, and produces a leaderboard comparable to Mariam et al. and the lechmazur benchmark.

**Dependencies:** Plan 01 (annotated archive with taxonomy + pattern labels). Plan 03 (agent harness). Plan 04 (run logs).

---

## Context

Mariam et al. (2024) established a 442-game baseline; the best LLM (Claude 3.5 Sonnet) solved ~18% fully. The lechmazur extended benchmark (940 games + trick words) now shows frontier models at 96–98%, but likely contaminated. The eval framework here:

1. Replicates Mariam et al. exactly (442-game subset, same metrics) for direct comparison
2. Extends to the full 1,090-game archive
3. Adds the trick-word distractor extension (borrowed from lechmazur)
4. Stratifies by taxonomy class and Wyna pattern
5. Imports Mariam et al. human baselines for apples-to-apples comparison

---

## Design Decisions

- **`eval` subcommand** in `connections-cli`: parallelizes across puzzles with configurable concurrency; checkpoints results so a partial run can resume.
- **Results storage**: one JSON file per `(model, puzzle_date, mode)` in `eval_results/`. Leaderboard aggregated from these files on demand.
- **Trick-word extension**: download lechmazur's extended dataset; run a separate `eval --trick-words` mode that injects distractors.
- **Human baselines**: import `game_with_knowledge_taxonomy.json` from Mariam et al. repo; parse expert/novice scores; store in `eval_results/human_expert.json` and `eval_results/human_novice.json`.

---

## Metrics

```rust
pub struct PuzzleResult {
    pub run_id: String,
    pub model: String,
    pub mode: EvalMode,
    pub puzzle_date: String,
    pub puzzle_id: u32,
    pub full_solve: bool,
    pub categories_solved: u8,    // 0–4
    pub guesses_used: u8,
    pub lives_used: u8,
    pub one_away_recoveries: u8,  // wrong guesses that were "one away" and later corrected
    pub taxonomy_results: [CategoryResult; 4],
    pub wyna_pattern_results: Vec<PatternResult>,
}

pub struct AggregateStats {
    pub full_solve_rate: f32,
    pub mean_categories_solved: f32,
    pub mean_guesses: f32,
    pub one_away_recovery_rate: f32,
    pub by_taxonomy: HashMap<String, f32>,   // taxonomy class → solve rate
    pub by_pattern: HashMap<String, f32>,    // wyna pattern → solve rate
}
```

---

## Implementation Steps

### 1. `eval` subcommand

```
connections eval \
  --model gpt-4o \
  --mode iterative \
  --since 2023-06-12 \
  --until 2026-06-05 \
  --concurrency 4 \
  --output eval_results/ \
  --skip-existing           # resume partial runs
```

- Iterates archive dates in range
- Spawns agent harness per puzzle (Plan 03 `run_puzzle()`)
- Writes `eval_results/YYYY-MM-DD_{model}_{mode}.json`
- Progress bar via `indicatif`

### 2. Human baseline import

```bash
connections eval import-baselines \
  --source game_with_knowledge_taxonomy.json \
  --output eval_results/
```

Writes `eval_results/human_expert.json` and `eval_results/human_novice.json` with same `PuzzleResult` schema.

### 3. Aggregate + leaderboard generation

```
connections eval leaderboard --input eval_results/ --output leaderboard.json
```

`leaderboard.json` schema:
```json
{
  "generated_at": "...",
  "entries": [
    {
      "model": "gpt-4o",
      "mode": "iterative",
      "n_puzzles": 1090,
      "full_solve_rate": 0.31,
      "mean_categories_solved": 3.1,
      "by_taxonomy": { "Semantic": 0.71, "Encyclopedic": 0.28, ... },
      "by_pattern": { "blank_fill": 0.55, "homophone_suffix": 0.19, ... }
    },
    ...
  ]
}
```

### 4. Trick-word distractor eval

Download lechmazur dataset; add `--trick-words` flag to `eval` subcommand. Each puzzle gets up to 4 injected distractor words; model must identify correct 4-word groups while ignoring them.

Run separately from main eval; stored in `eval_results/trick_words/`.

### 5. Stratification charts (data output)

`connections eval report --input eval_results/ --output report.json`

Produces aggregated data suitable for SVG chart rendering in Plan 08:
- Solve rate over time (by puzzle date, smoothed 30-day window)
- Per-taxonomy accuracy heatmap data
- Model comparison bar chart data

---

## Verification

```bash
# Single-model eval, 10-puzzle sample
connections eval --model gpt-4o --mode words-only --since 2024-01-01 --until 2024-01-10
ls eval_results/   # expect 10 files

# Leaderboard
connections eval leaderboard --input eval_results/ --output leaderboard.json
cat leaderboard.json | jq '.entries[0].full_solve_rate'

# Replication check
# Run on same 442 games as Mariam et al., expect gpt-4o ~30–38% (consistent with Todd et al.)
connections eval --model gpt-4o --mode words-only --since 2023-06-12 --until 2024-08-01
connections eval leaderboard | jq '.entries[] | select(.model == "gpt-4o") | .full_solve_rate'
```
