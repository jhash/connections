# Plan 03: Agent Harness

**Goal:** Provide a structured interface — MCP server and CLI harness — through which any language model can play Connections, with configurable evaluation modes, prompt templates, and optional RAG tooling.

**Dependencies:** Plan 01 (pattern index for RAG tool). Plan 02 (`connections-core` `GameState`).

---

## Context

The three evaluation modes from Mariam et al. (2024) define the baseline:
- **Words-only**: model sees 16 shuffled words once, produces 4 ordered guesses, no feedback
- **Iterative w/ hints**: model sees current board state after each guess, receives "one away" signal when applicable, up to 4 lives
- **Oracle**: on each correct guess, model is told which category it solved (upper bound on performance)

The MCP server exposes these as tools so any MCP-capable agent (Claude Code, Claude Desktop, custom agents) can play without bespoke integration.

---

## Design Decisions

- **MCP server**: implement with the [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) Rust SDK (official MCP Rust SDK). Runs as a stdio server, launchable via `connections mcp-server`.
- **CLI harness**: `connections solve --model gpt-4o --mode iterative --date 2026-06-05` — drives the model via API, records a structured run log. Used by the eval runner (Plan 05) and daily solver (Plan 06).
- **Prompt templates**: plain `.txt` files in `prompts/` with `{words}`, `{history}`, `{hint}` placeholders. Swappable without recompiling.
- **Model backends**: trait object `trait ModelBackend` with impls for OpenAI-compatible HTTP (covers OpenAI, Ollama, LM Studio) and Anthropic SDK.

---

## Implementation Steps

### 1. MCP server (`connections mcp-server`)

Tools exposed:

| Tool | Arguments | Returns |
|------|-----------|---------|
| `get_puzzle` | `date?: string` | `{ id, date, words: string[] }` (shuffled, no category info) |
| `submit_guess` | `words: [string, string, string, string]` | `{ result: "correct"\|"one_away"\|"wrong"\|"game_over", category?: string, lives_remaining: u8 }` |
| `get_hint` | — | `"one away"` if last guess was one-away, else `null` |
| `query_pattern_index` | `words: string[]` | `[{ pattern, examples, confidence }]` — top-3 pattern hypotheses |
| `get_board_state` | — | `{ remaining_words: string[], solved_categories: [{title, words}], lives: u8 }` |

Server maintains one `GameState` per session. Sessions keyed by a UUID passed in the `_meta` field or negotiated at init.

```
crates/connections-mcp/
  src/
    lib.rs
    server.rs   # MCP tool handlers
    session.rs  # session store (in-memory HashMap)
```

### 2. `ModelBackend` trait

```rust
pub trait ModelBackend: Send + Sync {
    fn complete(&self, prompt: &str) -> Result<String, ModelError>;
    fn name(&self) -> &str;
}

pub struct OpenAIBackend { endpoint: Url, model: String, api_key: String }
pub struct AnthropicBackend { model: String, api_key: String }
```

Config read from `models.toml` (see Plan 06 for full schema).

### 3. Prompt templates

`prompts/` directory:

```
prompts/
  words-only-zero-shot.txt
  words-only-cot.txt
  iterative-zero-shot.txt
  iterative-cot.txt
  few-shot-header.txt       # injected before examples
```

Template variables: `{words}`, `{history}`, `{one_away_hint}`, `{lives}`, `{examples}`.

Few-shot examples sampled from annotated archive at runtime (stratified by taxonomy class).

### 4. CLI harness (`connections solve`)

```
connections solve \
  --model gpt-4o \
  --mode iterative \        # words-only | iterative | oracle
  --date 2026-06-05 \
  --lives 4 \               # 0 = unlimited
  --prompt iterative-cot \  # template name
  --output run.json
```

Loop:
1. Build prompt from template + current board state
2. Call model backend
3. Parse model response → extract 4-word guess (regex + fallback: ask model to reformat)
4. Submit to `GameState`, record turn in run log
5. Repeat until solved or game over

Run log schema (see Plan 04 for full spec).

### 5. RAG integration

`query_pattern_index` MCP tool calls `src/index.rs` at runtime. In the iterative prompt template, add optional section:

```
You may call `query_pattern_index` with any subset of words to test a pattern hypothesis before guessing.
```

In the CLI harness, parse tool-call responses from the model (for models that support function calling) and route to the index.

---

## Verification

```bash
# Start MCP server and test with MCP inspector
connections mcp-server &
npx @modelcontextprotocol/inspector stdio -- connections mcp-server
# verify get_puzzle, submit_guess, get_board_state tools visible and functional

# CLI harness (words-only, single puzzle)
connections solve --model gpt-4o --mode words-only --date 2024-01-15 --output /tmp/run.json
cat /tmp/run.json | jq '.turns | length'   # expect 4 (one per group)
cat /tmp/run.json | jq '.solved'           # true or false

# Iterative mode with one-away recovery
connections solve --model claude-sonnet-4-6 --mode iterative --date 2024-03-20
# verify "one away" signal appears in log when applicable
```
