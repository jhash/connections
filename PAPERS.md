# Prior Work

Papers and benchmarks on LLM performance and tooling for the NYT Connections word game.

---

### Connecting the Dots: Evaluating Abstract Reasoning Capabilities of LLMs Using the New York Times Connections Word Game
*Samadarshi, Mustafa, Kulkarni, Rothkopf, Chakrabarty, Muresan — June 2024*
[arXiv:2406.11012](https://arxiv.org/abs/2406.11012) · [GitHub](https://github.com/mustafamariam/llm-connections-solver)

The foundational benchmark for this problem. Tests five frontier LLMs (Gemini 1.5 Pro, Claude 3.5 Sonnet, GPT-4o, Llama 3.1 405B, Mistral Large 2) on 438 games with few-shot and chain-of-thought prompting. Best result: Claude 3.5 Sonnet at **18% full-solve rate**, substantially below both novice and expert humans. Introduces a four-class knowledge taxonomy — **Semantic**, **Encyclopedic**, **Multiword Expression**, **Word Form + Meaning** — and shows models handle semantic categories far better than the rest. Dataset of 442 annotated games with human baselines is publicly available.

---

### NYT-Connections: A Deceptively Simple Text Classification Task that Stumps System-1 Thinkers
*Loredo Lopez, McDonald, Emami — December 2024 (COLING 2025 Best Dataset Paper)*
[arXiv:2412.01621](https://arxiv.org/abs/2412.01621)

The most methodologically rigorous of the group. 358 puzzles across six LLMs and human participants in three conditions: single attempt, multi-attempt without hints, multi-attempt with hints. Core argument: the game specifically resists **System-1 (fast, associative) reasoning** — models that rely on surface similarity fail disproportionately as difficulty increases. GPT-4 falls ~30% short of human performance even with best prompting. Designed to resist data leakage via regular updates. The System-1/System-2 framing maps directly onto the value of deliberate search tooling (pattern index, RAG) over pure next-token prediction.

---

### Missed Connections: Lateral Thinking Puzzles for Large Language Models
*Todd, Merino, Earle, Togelius — April 2024*
[arXiv:2404.11730](https://arxiv.org/abs/2404.11730)

Earlier NYU work that frames Connections as a **lateral thinking** benchmark rather than pure semantic retrieval. Tests sentence-embedding baselines (BERT, RoBERTa, MiniLM, MPNet) alongside GPT-3.5/4; GPT-4 with chain-of-thought reaches ~39% on their subset. Identifies that embeddings alone are fundamentally insufficient because category membership is often defined by non-semantic relations (wordplay, cultural knowledge, syntactic pattern). Motivates the need for the structured pattern index in this project.

---

### Making New Connections: LLMs as Puzzle Generators for the NYT Connections Word Game
*Merino, Earle, Sudhakaran, Sudhakaran, Togelius — July 2024*
[arXiv:2407.11240](https://arxiv.org/abs/2407.11240)

Flips the problem: can LLMs **generate** Connections puzzles rather than solve them? Uses Tree of Thoughts prompting with GPT models; human evaluators rate AI-generated puzzles as enjoyable and creative. Key insight: generation requires metacognition — the model must model how a human solver will reason, which is a dual of the solving task. Useful for generating synthetic training data: a model that can generate hard puzzles implicitly understands what makes them hard.

---

### lechmazur/nyt-connections (Extended Benchmark)
*Lechmazur — ongoing, last updated February 2026*
[GitHub](https://github.com/lechmazur/nyt-connections)

Live community benchmark, not a paper. 940 puzzles; extends each with up to four **trick words** that plausibly fit a category but don't — forcing models to reject attractive distractors. One attempt per puzzle, partial credit. As of early 2026, frontier models score 96–98% on the standard track (likely training-contaminated); the trick-word extension and a 100-puzzle recency-filtered subset are designed to resist saturation. The distractor design is worth borrowing for evaluation here, especially when testing smaller fine-tuned models.

---

*See [TODO.md](TODO.md) for how this prior work informs the evaluation framework and agent harness design.*
