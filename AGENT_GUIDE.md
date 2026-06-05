# Connections Solver — Agent Guide

Reference for solving NYT Connections (and clones like connectionsplus.io) with minimal wasted guesses.

## Objective

Sort 16 words into 4 groups of 4, where each group shares a hidden connection. Submit one group of 4 at a time. Goal: find all four groups with fewest mistakes.

## Core Rules

- 4×4 grid of 16 words/phrases.
- Exactly 4 hidden categories, each containing exactly 4 members. No exceptions.
- Select 4 tiles, submit. Game tells you if all 4 are correct (group locks, category name revealed) or incorrect.
- **"One away"**: many versions tell you when exactly 3 of your 4 are correct. Strong signal — swap one tile and retry.
- **Mistake limit**: standard NYT allows 4 total wrong guesses before game over. Clones vary (confirm the specific limit before playing). Treat guesses as scarce.
- No tile is ever shared between two categories. Every tile belongs to exactly one group. Mutual exclusivity is the most important solving constraint.

## Difficulty Color Scheme

NYT assigns each solved category a color signaling difficulty:

- 🟨 **Yellow** — easiest, most straightforward.
- 🟩 **Green** — easy-to-moderate.
- 🟦 **Blue** — moderate-to-hard.
- 🟪 **Purple** — hardest, usually the "trick" category (wordplay, hidden words, `___ + word`, puns, fill-in-the-blank).

Colors are revealed only *after* solving a group — but knowing purple is almost always wordplay shapes your hypothesis for the weirdest tiles.

## Knowledge Taxonomy

Research (Mariam et al. 2024) identifies four category types. Models handle them very differently:

| Type | Description | Model difficulty |
|------|-------------|-----------------|
| **Semantic** | Words share a meaning or topic (e.g., types of fish) | Easiest for models |
| **Encyclopedic** | Cultural, factual, or domain knowledge (films, athletes, brands) | Hard — recency-sensitive |
| **Multiword Expression** | Idioms, compound phrases, fill-in-the-blank | Hard |
| **Word Form + Meaning** | Wordplay, hidden words, homophones, anagrams | Hardest for models |

Knowing which type a category is helps calibrate confidence: a clean semantic group is safer to submit early; a wordplay group requires cracking the trick before submitting anything that might overlap with it.

## The System-1 Trap (most important concept)

NYT Connections is specifically designed to resist fast, associative reasoning. The first grouping that "feels obvious" is usually bait.

**Research finding (Loredo Lopez et al. 2024):** Models relying on surface similarity fail disproportionately as difficulty increases. GPT-4 falls ~30% short of human performance even with best prompting. The game requires deliberate System-2 reasoning — slow, structured, hypothesis-testing.

**The overlap trap:** if you can immediately list 5+ words for one theme, that theme is either fake or one of those words belongs elsewhere. The "extra" words are decoys with a second, less-obvious home — usually the real hard category.

**Rule of thumb:** don't submit the obvious group first. Find the *non-obvious* categories first (tight, unambiguous membership), submit those, and let elimination resolve the ambiguous overlaps.

## Common Trap Types

- **Decoy theme.** A cluster of 5–6 words all evoking one topic. The real categories steal members away one at a time. Always assume an obvious cluster is over-seeded.
- **Hidden words.** Purple categories often hide a word inside or at the end of a longer word. Scan odd/long words for embedded animals, body parts, vehicles, colors, numbers, names.
- **`___ + common word`.** Words that all precede or follow a shared word (e.g. all can precede "house": bird, light, ware, green). Or all follow it.
- **Homophones / sound-alikes.** Grouped by how they sound, not spelling.
- **Anagrams.** Letters rearrange into something shared.
- **Proper-noun trap.** A word that's both a common noun and a name/title. Board tiles are all uppercase so capitalization won't help.
- **Category-of-category.** Members are all examples of a set: cereal shapes, types of knots, Monopoly tokens, etc.
- **Compound / first-name / last-name.** Famous people sharing a first name; words that are all someone's surname.
- **Different senses of one word.** Tiles connected by a meaning you'd normally read another way.

## Solving Procedure (recommended order)

1. **Read all 16 first.** Don't lock onto anything yet.
2. **Note every obvious theme and count its candidates.** If a theme has more than 4 candidates, flag every candidate as contested. Do not submit a contested group.
3. **Hunt for the wordplay/purple category.** Look at the weirdest, longest, or most out-of-place words. Check for hidden words, anagrams, shared prefixes/suffixes. Cracking this usually frees up decoy words and collapses the rest.
4. **Find the tightest real category** — 4 words that fit one connection and have no plausible second home. Submit that first.
5. **Use mutual exclusivity.** Each tile assigned removes it from contention everywhere else. Resolve overlaps by asking "which category *needs* this word, vs. which merely *could* take it?"
6. **Solve by elimination last.** The final group is whatever remains — but verify it has a real shared connection before submitting.
7. **If unsure between two arrangements, submit the group you're most certain of**, never the one with a swappable member. Preserve guesses for genuinely ambiguous splits.

## Confidence Discipline

- Only submit a group when all 4 members are forced — either by a tight unique connection or by elimination.
- If two candidate words could each fill the last slot of two different groups, you haven't solved it yet. Keep reasoning; don't gamble.
- Beware overconfidence on pop-culture facts (film years, discographies, rosters). Recency-sensitive trivia is where models fail — verify if a guess hinges on it.
- A clean solve has zero wasted guesses *because* every group was deduced, not guessed. Treat any urge to "just try it" as a sign deduction isn't finished.

## Key Structural Lessons

These patterns recur across hard puzzles and should be checked explicitly:

- **Every obvious decoy cluster is a signal, not an answer.** The words in it probably scatter across 2–3 real categories. Name the decoy, then ask where each of its members *actually* belongs.
- **Crack the wordplay category first.** It's almost always the key that unlocks the rest. A hidden word, homophone, or fill-in-the-blank pattern recontextualizes tiles you thought you understood.
- **Submit tight categories early, ambiguous categories late.** Elimination is your friend — it resolves overlaps that reasoning alone can't.
- **When 4 words remain after solving 3 groups, verify they share a real connection.** If they don't, you made an error earlier. Don't submit a nonsense final group.

## Community Puzzles (connectionsplus.io)

Community-authored puzzles (e.g. by chloetron, jaycub) diverge from NYT Wyna style in ways that matter:

- Category names and difficulty bands may not follow NYT conventions.
- Wordplay may be more or less forgiving than the NYT purple.
- Pop-culture references may skew toward the author's specific knowledge domain.

Apply the same solving strategy, but calibrate expectations: the puzzle may be harder, easier, or stylistically different from the NYT baseline.

## Available Tools (Agent Harness)

When playing through the MCP harness (see TODO.md §3), these tools may be available:

- `get_puzzle` — fetch puzzle words and metadata.
- `submit_guess` — submit a group of 4; returns correct/incorrect and "one away" signal.
- `get_hint` — request a hint (may not always be available depending on harness config).

In **iterative mode**, use the running guess history and "one away" signals as hard constraints when forming new hypotheses. In **words-only mode**, you get no feedback — every guess must be a confident deduction.
