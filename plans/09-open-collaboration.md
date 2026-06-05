# Plan 09: Open Collaboration

**Goal:** Lower the barrier for external contributors to add models, run evals, and extend the harness — and publish the fine-tuning dataset and methodology for the wider research community.

**Dependencies:** Plan 03 (agent harness API). Plan 05 (eval framework). Plan 07 (fine-tuning dataset, ≥200 traces before HuggingFace publish).

---

## Context

The value of this project scales with participation. If other researchers can plug in their own models and get standardized results, the leaderboard becomes a living benchmark rather than a one-time snapshot. The open-source angle also positions this work as a natural extension of Mariam et al. — a maintained, growing benchmark with tooling.

---

## Design Decisions

- **CONTRIBUTING.md**: step-by-step instructions for the three most common contributor actions: adding a model, running the eval, submitting results.
- **Fabro evaluation**: [Fabro](https://github.com/fabro-sh/fabro) is a structured agent workflow tool. Evaluate it against a custom hint-loop implementation; decide within one sprint (≤2 weeks of use).
- **HuggingFace dataset**: publish when finetune dataset reaches ≥200 annotated traces. Use the `datasets` Python library push API rather than manual upload.
- **Paper**: write as a short arXiv note (not a full conference submission initially) tracking deltas from each intervention vs. Mariam et al. baseline.

---

## Implementation Steps

### 1. `CONTRIBUTING.md`

Sections:
1. **Adding a model** — add entry to `models.toml`, verify with `connections solve --model <id> --date 2024-01-15`
2. **Running the eval** — `connections eval` command, expected output, how to interpret leaderboard
3. **Submitting results** — open a PR adding your `eval_results/` files; CI validates schema
4. **Extending prompt templates** — add file to `prompts/`, test with `connections solve --prompt <name>`
5. **Community puzzles** — add username to `community/` watch list; note that community authors may not follow NYT style conventions

### 2. CI validation for contributed eval results

`.github/workflows/validate-results.yml`:
- On PR, run `connections eval validate --input eval_results/` to check schema, no duplicates, plausible metrics
- Prevents garbage results from inflating/deflating leaderboard

### 3. Fabro evaluation

Trial period: 2 weeks using Fabro for the human↔agent hint workflow loop.

Criteria for keeping Fabro:
- Reduces harness code by ≥30%
- Works with stdio MCP server without modification
- Handles the iterative mode's "one away" signal flow naturally

Criteria for replacing with custom loop:
- Too opinionated about conversation structure
- Adds dependency overhead not justified by simplification

Document decision and rationale in `docs/fabro-evaluation.md`.

### 4. HuggingFace dataset publish

Trigger: when `finetune/traces/` contains ≥200 files.

```bash
scripts/publish_dataset.py \
  --input finetune/traces/ \
  --repo jhash/connections-reasoning-traces \
  --split train
```

Dataset card (`finetune/README.md`) includes:
- Collection methodology (distilled from GPT-4o/Claude Opus)
- Schema description
- Taxonomy label distribution
- License (CC BY 4.0)
- Citation for Mariam et al. as upstream data source

### 5. Paper draft

`paper/` directory, Markdown source compiled to PDF via Pandoc (or LaTeX if journal submission needed).

Structure:
1. Introduction — Connections as abstract reasoning benchmark; gap from Mariam et al.
2. Related Work — cite all papers in PAPERS.md
3. System — archive pipeline, annotation, pattern index, agent harness
4. Experiments — ablation (base, +RAG, fine-tuned, fine-tuned+RAG); learning curve; trick-word eval
5. Results — delta vs. Mariam et al. baseline; taxonomy stratification; small-model performance
6. Discussion — System-1 vs. deliberate reasoning framing; community puzzle style divergence; limitations
7. Conclusion + future work

Publish to arXiv once eval results cover ≥3 models and ≥500 puzzles.

---

## Verification

```bash
# CONTRIBUTING.md walkthrough — add a new Ollama model end-to-end
# (follow the doc; if anything is unclear, fix the doc)

# CI schema validation
connections eval validate --input eval_results/
# expect: 0 errors

# Fabro trial (manual)
# Run 10 iterative sessions through Fabro; measure code reduction vs. current harness

# HuggingFace dry run
python scripts/publish_dataset.py --input finetune/traces/ --dry-run
# expect: dataset card preview, file count, schema validation pass

# Paper draft compiles
cd paper && pandoc README.md -o paper.pdf
```
