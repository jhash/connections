# Plan 07: Fine-tuning

**Goal:** Fine-tune small open-weight models (≤7B parameters) to maximize Connections solve rate, using chain-of-thought reasoning traces distilled from larger models, with daily incremental updates and a rigorous ablation comparing RAG vs. baked knowledge.

**Dependencies:** Plan 01 (annotated archive — taxonomy labels improve trace quality). Plan 05 (eval framework — measures improvement). Plan 03 (agent harness — generates traces).

---

## Context

The key finding from Loredo Lopez et al. (COLING 2025) is that even GPT-4 falls ~30% short of humans specifically on *deliberate* reasoning tasks — the game resists System-1 shortcuts. Fine-tuning on reasoning traces (not just correct answers) is the primary hypothesis for closing this gap on small models.

Target models chosen for size/capability tradeoff and Candle support:
- **Qwen2.5-1.5B**: smallest viable; goal is consistent ≥50% full-solve
- **Qwen2.5-7B**: primary target; aim to match or exceed Mariam et al. Claude 3.5 Sonnet baseline (18%) within first 300-puzzle training set, then push toward human expert (~48%)
- **Llama-3.2-3B**: alternate architecture for comparison

---

## Design Decisions

- **Training method**: supervised fine-tuning (SFT) on `(prompt, trace+answer)` pairs. LoRA adapters to keep memory footprint small.
- **Primary pipeline**: Candle (Rust) — aligns with project's Rust-first ethos, runs on CPU/Metal/CUDA.
- **Fallback**: `scripts/train.py` using HuggingFace `trl` SFTTrainer + `peft` LoRA — better tooling maturity, used if Candle support for a model is incomplete.
- **Trace distillation**: use GPT-4o or Claude Opus to generate reasoning traces for solved puzzles. Prompt: "explain step by step why these four words go together and why you chose this group first."
- **Dataset format**: ShareGPT-style JSON (compatible with both Candle and trl).

---

## Dataset Schema

`finetune/traces/YYYY-MM-DD.json` per puzzle:

```json
{
  "puzzle_date": "2024-01-15",
  "puzzle_id": 512,
  "conversations": [
    {
      "from": "human",
      "value": "Group these 16 words into 4 categories of 4:\nWITCH OVEN FOREST COTTAGE KNIGHT LANCE SHIELD JOUST BASS TREBLE CLEF NOTE RUBY EMERALD SAPPHIRE DIAMOND\n\nThink step by step."
    },
    {
      "from": "gpt",
      "value": "Let me work through this systematically...\n\n[reasoning trace]\n\nMy guesses in order:\n1. WITCH, OVEN, FOREST, COTTAGE — HANSEL AND GRETEL\n2. KNIGHT, LANCE, SHIELD, JOUST — MEDIEVAL COMBAT\n3. BASS, TREBLE, CLEF, NOTE — MUSIC\n4. RUBY, EMERALD, SAPPHIRE, DIAMOND — GEMSTONES"
    }
  ],
  "taxonomy_labels": ["Encyclopedic", "Semantic", "Semantic", "Semantic"],
  "wyna_patterns": ["cultural_reference", "semantic_field", "semantic_field", "semantic_field"]
}
```

### Daily increment

`run-daily.sh` extension (after eval block):
```bash
"$BINARY" distill-trace --date yesterday \
  --model gpt-4o \
  --output "$PROJECT_DIR/finetune/traces/"
```

### 2. `distill-trace` subcommand

- Fetches yesterday's puzzle from archive
- Calls large model with distillation prompt
- Parses trace + answer
- Validates answer matches known solution
- Writes to `finetune/traces/YYYY-MM-DD.json`
- Skips if file exists (idempotent)

### 3. Candle training pipeline

`crates/connections-train/src/main.rs`:

```
connections-train \
  --model qwen2.5-7b \
  --dataset finetune/traces/ \
  --output models/qwen2.5-7b-connections-v1/ \
  --lora-rank 16 \
  --epochs 3 \
  --batch-size 4
```

Steps:
1. Load base model weights (auto-download from HuggingFace hub via `hf-hub` crate)
2. Apply LoRA adapters to attention layers
3. Tokenize dataset (ShareGPT format → instruction-response pairs)
4. Training loop with gradient accumulation
5. Save adapter weights to `models/` dir

Candle crate dependencies: `candle-core`, `candle-nn`, `candle-transformers`, `hf-hub`.

**Fallback (`scripts/train.py`)**:
```python
# Uses trl SFTTrainer + peft LoRA
# Invoked if Candle support incomplete for target model
python scripts/train.py \
  --model Qwen/Qwen2.5-7B-Instruct \
  --dataset finetune/traces/ \
  --output models/qwen2.5-7b-connections-v1/
```

### 4. Ablation study

Four conditions, evaluated with Plan 05 framework:

| Condition | Config |
|-----------|--------|
| A: Base | `--model qwen2.5-7b --no-rag` |
| B: Base + RAG | `--model qwen2.5-7b --rag pattern-index` |
| C: Fine-tuned | `--model models/qwen2.5-7b-v1 --no-rag` |
| D: Fine-tuned + RAG | `--model models/qwen2.5-7b-v1 --rag pattern-index` |

Run all four on same puzzle set; compare full-solve rates in leaderboard.

### 5. Learning curve tracking

```bash
connections eval learning-curve \
  --model qwen2.5-7b \
  --checkpoint-dir models/checkpoints/ \
  --checkpoint-every 50 \   # re-eval every 50 training examples
  --output learning_curve.json
```

Generates data for a line chart: x = training set size, y = full-solve rate. Stored in `eval_results/learning_curves/`.

---

## Verification

```bash
# Trace distillation
connections distill-trace --date 2024-01-15 --model gpt-4o
cat finetune/traces/2024-01-15.json | jq '.conversations[1].value' | head -20
# expect: reasoning trace ending with 4 ordered guesses

# Candle training (small test run)
connections-train --model qwen2.5-1.5b --dataset finetune/traces/ --output /tmp/test-model/ --epochs 1 --max-samples 10
# expect: loss decreasing, adapter saved

# Eval on fine-tuned model
connections eval --model /tmp/test-model --mode words-only --since 2024-01-01 --until 2024-01-10
connections eval leaderboard | jq '.entries[] | select(.model | contains("test-model"))'
```
