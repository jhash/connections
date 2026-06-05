# Plan 01: Data Pipeline

**Goal:** Produce a fully annotated puzzle corpus — taxonomy labels, Wyna pattern tags, pop-culture context, and community author profiles — that downstream plans (eval, fine-tuning, web) can consume directly.

**Dependencies:** None. This is the foundation everything else builds on.

---

## Context

Raw `archive.json` (~1,090 NYT puzzles) already exists. Community archives (`chloetron.json`, `jaycub.json`) exist but have no `categories` yet (encrypted). The annotation work here adds structured metadata without modifying any existing fields — purely additive schema changes.

Mariam et al. established a four-class knowledge taxonomy in `game_with_knowledge_taxonomy.json` (442 games). The goal is to extend that coverage to the full archive and add a finer-grained Wyna-pattern layer on top.

---

## Design Decisions

- **Annotation storage**: add a `taxonomy` field (enum string) per `Category` in the archive schema. Add a `wyna_pattern` string field per `Category` for finer-grained labels. Null = unannotated; avoids breaking existing consumers.
- **Taxonomy annotation method**: LLM-assisted batch labeling (GPT-4o or Claude) via a `connections annotate` subcommand; human spot-check sample. Import the 442 Mariam et al. labels directly rather than re-labeling.
- **Pattern index**: [Tantivy](https://github.com/quickwit-oss/tantivy) (pure Rust, embedded) over Elasticsearch — no external process, deploys as part of the binary.
- **Pop-culture context**: Wikipedia API (`/api/rest_v1/page/summary/{title}`) for structured event lookup; cache results in `context_cache/YYYY-MM-DD.json` to avoid re-fetching.
- **Community profiles**: extend the `user-archive` subcommand output, not a separate pipeline.

---

## Implementation Steps

### 1. Extend archive schema
- Add `taxonomy: Option<String>` and `wyna_pattern: Option<String>` to `Category` in `src/main.rs` (both `#[serde(default, skip_serializing_if = "Option::is_none")]`)
- Verify round-trip: re-serialize `archive.json`, diff should be empty

### 2. Import Mariam et al. taxonomy labels
- Download `game_with_knowledge_taxonomy.json` from [mustafamariam/llm-connections-solver](https://github.com/mustafamariam/llm-connections-solver)
- Write a one-off migration script (`scripts/import_taxonomy.py` or inline Rust) that matches by puzzle `id`, copies taxonomy label to each category
- Run against `archive.json`, commit updated archive

### 3. LLM batch annotation for unlabeled categories
- New subcommand: `connections annotate --model gpt-4o --output archive.json`
- Reads unannotated categories, batches into prompts (system prompt describes the 4-class taxonomy with examples from the paper), writes labels back
- Rate-limit: 10 req/s, resume on restart (skip already-labeled)

### 4. Wyna pattern taxonomy
Define and document known patterns in `docs/wyna-patterns.md`:

| Pattern | Example category | Rule |
|---------|-----------------|------|
| `homophone_suffix` | "Sounds like ___" | words that sound like another word when a suffix is added |
| `blank_fill` | "___ BALL" | words that complete a compound with a common word |
| `minus_last_letter` | "Disney princesses - last letter" | truncated proper nouns |
| `transport_mode` | "Ways to get there" | vehicles / transit methods |
| `wordplay_compound` | "FIRE ___" | one-word prefix/suffix chains |
| `semantic_field` | "Types of cheese" | pure semantic grouping |
| `cultural_reference` | "Taylor Swift albums" | encyclopedic |

- Implement rule-based pattern detector in `src/patterns.rs` — heuristics first, LLM fallback for ambiguous cases
- Add `connections tag-patterns --output archive.json` subcommand

### 5. Pattern index (Tantivy)
- Add `tantivy` dependency to `Cargo.toml`
- `src/index.rs`: `build_index(archive: &[Puzzle]) -> Index`; fields: `date`, `category_title`, `words` (multi-value), `taxonomy`, `wyna_pattern`
- `connections index build` — writes index to `.index/` dir
- Expose `connections index query --words "WITCH,OVEN,FOREST,COTTAGE" --top 5` for manual testing; agent harness will call this as a library function

### 6. Pop-culture context cache
- `connections context fetch --since 2023-06-12` — for each puzzle date, query Wikipedia for top events on that date, store in `context_cache/YYYY-MM-DD.json`
- Schema: `{ date, events: [{ title, summary, url }] }`
- `daily archive` job extended to fetch context for new puzzles

### 7. Community profile style analysis
- In `cmd_user_archive`: after fetching, compute pattern distribution (once categories are decryptable) and compare to NYT baseline distribution
- Store as `style_profile: { pattern_counts: {..}, vs_nyt_baseline: {..} }` in the archive root metadata
- For now, document structure; populate after decryption is solved (Plan 03)

---

## Verification

```bash
# Schema round-trip
cargo test -- --test-output immediate

# Taxonomy import
jq '[.[].categories[].taxonomy] | map(select(. != null)) | length' archive.json
# expect: 442 * 4 = 1768 labeled categories after import

# Pattern index
connections index build
connections index query --words "WITCH,OVEN,FOREST,COTTAGE"
# expect: top results include categories with homophone/compound patterns

# Context cache
ls context_cache/ | wc -l
# expect: ~1090 files
```
