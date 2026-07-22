# cardcrusher-rs — Project Plan

A from-scratch, Rust reimagining of the `ygopro-core` rules engine, with a new,
readable card DSL in **Lua** (embedded via `mlua`). The DSL was first prototyped
on Rhai, then moved to Lua for real coroutines — see [PHASE-LUA.md](PHASE-LUA.md).
This document captures **Phase 1** in detail and gives an honest estimate for the
whole effort.

> Background and rationale for every design decision here live in the
> architecture book at `edopro/docs/engine-book/` — chapter references below
> (e.g. "Ch 5") point there.

---

## The two north-star decisions

1. **Arena + IDs, not pointers.** Every object lives in a `SlotMap`; every
   `card*`/`effect*`/`group*` becomes a `Copy` key (`CardId`, …). Generational
   keys give us memory-safe references **and** the "deleted object" tombstone for
   free (Ch 3, Ch 9 §9.2).
2. **Separate the IR from the authoring surface.** The engine's stable contract
   is the effect IR (type + range + condition + cost + target + operation + value
   + reset + count-limit — Ch 7). Any front-end (Lua now, a custom parser later)
   just emits IR. Design the IR well; the DSL is swappable — as the Rhai→Lua
   move itself proved.

Determinism is a constraint baked in from day one (Ch 13): one seeded PRNG
(`rand_xoshiro::Xoshiro256StarStar`, matching the reference), integer-only game
logic, sort-before-iterate, IDs not pointers, and **`BTreeMap`/sorted `Vec` — never
`HashMap` — in any state-affecting path.**

---

## Phase 1 — "Skeleton + the two risky spikes"

**Goal:** prove the *architecture*, not the gameplay. By the end, all three hard
subsystems (data model, state machine, DSL) talk to each other — with **zero**
actual Yu-Gi-Oh! rules implemented. De-risk before investing.

| # | Milestone | Exit criteria | Rough effort |
|---|-----------|---------------|--------------|
| 1 | **Crate skeleton & guardrails** | modules laid out; `cargo build/test/clippy/fmt` green; determinism rules documented | days |
| 2 | **Data model (arena + IDs)** | `Duel` owns `SlotMap`s for cards/effects/groups; a stale `CardId` after delete returns `None` (tombstone proven by a test) | ~1 wk |
| 3 | **Processor state machine** | pop→run→push-back driver mirroring `processor_visit.cpp` (Ch 5); a `Startup → Turn` stub runs to `End` | ~1 wk |
| 4 | **Freeze / resume** | a `SelectCard`-style unit returns `Awaiting`, host sets a response, unit resumes reading it — round-trip test passes | ~few days |
| 5 | **⭐ The DSL spike** (the real risk) | the card's entry fn runs, produces an `EffectDef` IR value, and its **deferred `resolve` closure is callable later** and fires a `Destroy` | 1–2 wks |
| 6 | *(stretch)* **Diff-test harness** | stub that feeds the same seed+responses to cardcrusher *and* ygopro-core and compares message bytes | ~1 wk |

**Definition of done:** one integration test boots a scripted mini-duel, runs the
`Example` card *through its script*, and observes the destroy end-to-end —
data model + state machine + DSL all connected.

**Explicitly OUT of scope for Phase 1:** turns, phases, real zones, chains,
battle, real cards. All Phase 2+.

> **Why this ordering:** milestone 5 is the only thing that can *kill the
> project*. Everything before it exists to make the spike runnable. If the DSL's
> deferred-closure model doesn't work as hoped, find out in week 3, not month 6.

---

## Later phases (sketch)

- **Phase 2 — Turns & operations.** Turn/phase clock, main-phase menu, the
  movement primitives (`SendTo` and friends), draw/damage. ~10–20 hand-built
  cards playable in a stub loop. (Ch 5, Ch 10)
- **Phase 3 — The chain engine.** Events, SEGOC ordering, quick-effect priority
  windows, LIFO resolution, the adjust loop. The hardest part. (Ch 8, Ch 11)
- **Phase 4 — Battle & rules variants.** Attack declaration, damage-step
  sub-timings, master-rule flag variants. (Ch 10 §10.9, Ch 4 §4.2)
- **Phase 5 — Card coverage.** Broad authoring via the DSL. Open-ended.

---

## How long the whole thing takes (honest)

This is really **two projects**:

**① The engine (bounded, hard).** Solo, part-time-serious (full-time ~2–3× faster):

- Phase 1: **2–6 weeks**
- Phase 2: **2–4 months**
- Phase 3 (chain/SEGOC — where solo engines die): **3–6+ months**, buggy for a while after
- Phase 4: **2–4 months**

→ A solid engine playing a meaningful slice of the game correctly: **~1–2 years part-time.**

**② The card content (unbounded).** ygopro-core is 10+ person-years of engine
work *plus* ~13,000 community scripts, and the real game adds cards monthly. With
a new DSL you re-author cards — a permanent pipeline, not a finish line.

> **True ygopro-core parity is not a solo goal.** It's community-scale and
> multi-year. Don't measure against it.

### The lever that makes it finite: narrow the scope

Target a specific cardpool — a **format** (GOAT, Edison), or a **curated
cube / your own 100–300 cards**. "All of Yu-Gi-Oh!" becomes "these 200 cards,"
and the project is **shippable in 6–18 months** as a fun, correct engine.

### Strategic fork

- **Want lots of real cards soon?** Keep Lua-API compat → instant 13k-card
  library; layer the nice DSL for *new* authoring.
- **The DSL + engine *is* the craft?** Timeline is the engine + a curated pool —
  a great, finite hobby project.

**Bottom line:** Phase 1 (~1 month) de-risks everything. A scoped, playable
engine is a realistic ~1–2 year part-time goal. Define scope *before* writing
engine code.
