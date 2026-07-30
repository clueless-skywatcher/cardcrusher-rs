# cardcrusher — working instructions

A from-scratch **Yu-Gi-Oh! rules engine in Rust**, with a **Lua card DSL** (via
the `mlua` crate). It's a learning-oriented reimagining of EDOPro's `ygopro-core`.

New here? Read **`HANDOVER.md`** — it recaps everything built so far and lays out
the current next phase (the battle system).

---

## ⭐ How to communicate (applies to EVERY reply and EVERY doc)

The user is **ADHD** and learning the engine as it's built. Optimize hard for
**scannable, low-jargon, concrete** communication.

- **Lead with the answer.** First sentence = the point. No preamble.
- **Short chunks.** Bullets over paragraphs. One idea per bullet. Cut ruthlessly.
- **Plain words, not jargon.** If a term is unavoidable, define it in ½ a line.
- **Show, don't lecture.** When explaining anything non-trivial, include:
  - a tiny **code snippet** or **diff** (before → after), and/or
  - a **dry run** — a step-by-step trace with real example values ("Hand: [A, B]
    → summon A → Field: [A], Hand: [B]"), and/or
  - a **code simulation** — walk the actual functions with concrete inputs.
- **Analogies rarely** — only when they genuinely click. Don't sprinkle them.
- Keep messages to ~2 short sections max. If it's longer, it's too long.
- Detail lives in **docs**, not chat. Chat is for the decision + the gist.
- Match this in docs too: headers, bullets, diffs, and dry-run examples.

> The user has said, more than once and emphatically, that wordy/jargon-heavy
> replies lose them. Take it seriously.

---

## How we work

- **TDD, red-first.** For a new behavior, write the *failing test* first, explain
  **what** it needs and **why**, then implement (or hand the spec to the user —
  they often implement and have Claude review). Keep `cargo test`, `cargo clippy
  --all-targets`, and `cargo fmt --check` green at every step.
- **Read the user's current code before advising** — they edit between turns and
  dislike stale assumptions.
- **Mirror EDOPro when unsure.** The reference C++ engine is at
  `~/Github/edopro/ocgcore/` and its architecture book at
  `~/Github/edopro/docs/engine-book/`. Read the real source and cite `file:line`.
- **Be precise on Yu-Gi-Oh! rulings** — verify against EDOPro, don't guess.
- **Determinism is sacred** (see `src/lib.rs`): one seeded PRNG, integers only,
  sort-before-iterate, IDs not pointers, `BTreeMap`/sorted `Vec` never `HashMap`.
  The Lua VM has its GC stopped for the same reason.

## The layout in one breath

- `src/duel/` — the `Duel` (owns everything), split into `mod.rs` (struct + Lua
  setup), `board.rs` (arena/zones/LP/wins), `driver.rs` (the processor loop +
  menu), `scripting.rs` (running Lua effects), and `prelude.lua` (the card DSL
  base classes + verbs).
- `src/effect.rs` — the `EffectContext` scratchpad + the Rust "verb" hooks Lua
  calls + `EffectKind`/`CostType`.
- `cards/*.lua` — one card per file, self-registering.
- `examples/play.rs` — a terminal hotseat demo (`cargo run --example play`).
