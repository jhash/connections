# connections+ — Research Roadmap

Inspired by [Mariam et al. (2024)](https://arxiv.org/abs/2406.11012), which establishes that even Claude 3.5 Sonnet solves only ~18% of NYT Connections games fully, while expert humans far exceed that. The goal here is to push that number as high as possible on the smallest possible models, with open tooling for others to do the same.

---

## Guide to This Repo

### File map

| File / folder | What it is | Who reads it |
|---------------|-----------|--------------|
| `README.md` | Project intro, CLI usage, setup | You |
| `TODO.md` (this file) | Roadmap with checkboxes | You |
| `PAPERS.md` | Prior research with summaries | You |
| `plans/` | Detailed implementation specs for each section | You + AI agents when building |
| `src/main.rs` | All current Rust code (CLI) | You when learning Rust |
| `archive.json` | Every NYT puzzle ever (~1,090) | Code consumes it |
| `chloetron.json`, `jaycub.json` | Community puzzle archives | Code consumes it |
| `scripts/` | Shell scripts for daily automation | You run these once to set up |
| `Cargo.toml` | Rust dependency list | Rust toolchain reads it |

**`plans/` are for both you and agents.** Read them before starting a section — they explain *why* each decision was made, which is useful context whether you're coding yourself or directing an agent. The detail level assumes you'll look up unfamiliar Rust/ML concepts as you go; nothing in them requires prior experience to follow.

### Build order

The sections depend on each other. Here's the order that makes sense to work through, and why:

```
1. Data  ←  everything downstream needs an annotated archive
    ↓
3. Agent Harness  ←  needed before you can run any evals or watch agents play
    ↓
5. Evaluation  ←  measures whether changes actually help
    ↓
6. Daily Solver  ←  automates the eval loop once it works manually
    ↓
7. Fine-tuning  ←  needs eval to measure improvement
```

In parallel with any of the above (no hard dependencies):
```
2. Interfaces (TUI + Web)  ←  standalone, good for learning Rust hands-on
4. Visualization  ←  depends on agent harness producing run logs, otherwise standalone
8. Web Service  ←  assembles everything into a single app; build last
9. Open Collaboration  ←  documentation and publishing; ongoing, not a code milestone
```

**Recommended starting point if you're new to Rust:** Section 2 (TUI). It's self-contained, has immediate visual feedback, and teaches the core Rust patterns (structs, enums, match, traits) you'll need everywhere else. `plans/02-interfaces.md` walks through it step by step.

**Recommended starting point if you want to see AI results fast:** Section 3 (Agent Harness) + Section 5 (Eval). Gets a model playing the game and scoring it within a few days of work.

### How to use the plans

Each file in `plans/` covers one section. Structure is always:
- **Goal** — one sentence saying what you're building
- **Dependencies** — what needs to exist first
- **Design decisions** — why the code is structured the way it is (read this before starting)
- **Implementation steps** — numbered, in order, concrete
- **Verification** — shell commands to confirm it's working

The plans reference Rust crates (libraries) by name. When you encounter an unfamiliar one, [docs.rs](https://docs.rs) has documentation for every published Rust crate.

---

## 1. Data

- [x] Archive all NYT puzzles (`archive` subcommand, ~1,090 games, back to 2023-06-12)
- [x] Archive community puzzles by username (`user-archive` subcommand)
- [ ] Annotate archive with knowledge taxonomy from the paper: **Semantic**, **Encyclopedic**, **Multiword Expression**, **Word Form + Meaning**
- [ ] Categorize finer-grained Wyna patterns (e.g. homophones appended, blank `___` fill-ins, modes of transport, Disney princesses minus last letter, "___ + common word" compounds)
- [ ] Build a structured pattern index (Elasticsearch or Tantivy) to embed words in a richer vector space — surface which pattern applies before solving
- [ ] Track pop-culture recency signals per puzzle (publication date → pull relevant events for RAG context)
- [ ] Connections+ community puzzle profiles — start with known authors (chloetron, jaycub), expand; note that community authors often diverge from Wyna's internal style conventions
- [ ] Diff community style vs. NYT style — document where they agree and diverge

---

## 2. Interfaces

### TUI
- [ ] Playable terminal UI (Ratatui) — render 4×4 grid, accept guesses, show color difficulty bands, track lives (default 4)
- [ ] Replay mode — step through a solved archive game

### Web UI (plain Rust stack: Axum + Maud + HTMX)
- [ ] Same gameplay as TUI, served over HTTP — this is also the live showcase surface (see §6)
- [ ] Game picker by date or community author
- [ ] Live agent visualization (see §4)

---

## 3. Agent Harness

Implement the three evaluation modes from the paper:

| Mode | Input given to model |
|------|----------------------|
| **Words-only** | 16 shuffled words, no feedback |
| **Iterative w/ hints** | Words + running guess history + "one away" signals when applicable |
| **Oracle** | Words + which category each correct guess falls into (upper bound) |

- [ ] MCP server exposing `get_puzzle`, `submit_guess`, `get_hint` tools — lets any MCP-capable agent play without bespoke integration
- [ ] Prompt templates: zero-shot, chain-of-thought, few-shot (sample from archive)
- [ ] Lives toggle (4 lives default, unlimited option for ablation)
- [ ] Pattern-index RAG tool — agent can query the index mid-solve to test a hypothesis (e.g. "are any of these homophones?")

---

## 4. Visualization

- [ ] Step-by-step agent replay as structured log (JSON: guess, confidence, reasoning)
- [ ] HTMX-driven live watch page — streams agent turns as server-sent events
- [ ] Export to video (headless browser → ffmpeg, or Ratatui → terminal recorder)
- [ ] Comparative run view: two agents solving same puzzle side-by-side

---

## 5. Evaluation Framework

Replicates and extends the Mariam et al. benchmark.

- [ ] Automated eval runner — iterates full NYT archive + selected community puzzles, records outcomes per model
- [ ] Metrics: full-solve rate, category-level accuracy, avg guesses to solve, "one away" recovery rate
- [ ] Stratify results by knowledge taxonomy label (which category types break each model)
- [ ] Stratify by Wyna pattern (do models handle homophones worse than fill-in-the-blank?)
- [ ] Human baseline import from paper dataset (442 games, expert + novice) for apples-to-apples comparison
- [ ] Leaderboard output (static JSON + rendered table on web UI)

---

## 6. Daily Solver Service

- [x] Daily archive job (launchd / systemd / Docker)
- [ ] Daily solver job — on new puzzle publish, run each configured model, record result
- [ ] Publish results to leaderboard; commit to repo or POST to web service
- [ ] Model registry config (local Ollama endpoints + remote API keys) — easy to add new models

---

## 7. Fine-tuning

Goal: maximize solve rate on minimum parameters (local-first).

- [ ] Supervised fine-tuning dataset — solved games annotated with reasoning traces (chain-of-thought distilled from large model)
- [ ] Candle (Rust) training pipeline; fallback to `transformers` + `trl` if needed
- [ ] Base model targets: Qwen2.5-1.5B, Qwen2.5-7B, Llama-3.2-3B — chosen for size/capability tradeoff
- [ ] Daily fine-tune increment — previous day's puzzle + answer added to training set automatically
- [ ] RAG vs. fine-tuning ablation — measure how much the pattern index helps vs. baking knowledge into weights
- [ ] Eval after each fine-tune increment; track learning curve over archive

---

## 8. Live Web Service

Single Axum app, Maud templates, HTMX — no JS framework.

- [ ] `/play` — human-playable game (date picker or random)
- [ ] `/agents` — leaderboard, per-model solve stats, taxonomy breakdown charts
- [ ] `/watch/:run_id` — live or recorded agent replay (SSE)
- [ ] `/archive` — browsable puzzle archive with metadata + taxonomy labels
- [ ] `/community/:username` — author profile, game list, style analysis vs. NYT baseline
- [ ] Graphs: solve rate over time, per-category accuracy heatmap, model comparisons (use a lightweight charting lib or render SVG server-side)

---

## 9. Open Collaboration

- [ ] Document the agent harness API so external contributors can plug in their own models
- [ ] Consider [Fabro](https://github.com/fabro-sh/fabro) for structured human↔agent hint workflows — evaluate whether it fits or if a custom loop is simpler
- [ ] Publish fine-tuning dataset to HuggingFace once large enough to be useful
- [ ] Write up methodology as a short note extending Mariam et al. — track delta in solve rates from pattern index + fine-tuning

---

## Reference

See [PAPERS.md](PAPERS.md) for full citations and descriptions of prior work, including Mariam et al. (2024), Loredo Lopez et al. (COLING 2025), Todd et al. (2024), Merino et al. (2024), and the lechmazur extended benchmark.
