# Handoff — cardcrusher-rs

Self-contained state for picking up in a fresh session. **For chain work, read
`docs/chain.md` first** — it's the living spec.

## Project
- **Repo:** `/home/epsilonator/Github/cardcrusher-rs`, branch `master`, remote
  `origin` (github: clueless-skywatcher/cardcrusher-rs). A from-scratch
  **Yu-Gi-Oh! rules engine in Rust** + **Lua card DSL (mlua)**, reimagining EDOPro's
  ygopro-core.
- **State:** 81 tests green, clippy 0, fmt clean.

## ⚠️ Working mode (changed — read this)
- **Hands-off for chain design.** The assistant gives **ideas + Socratic questions
  only — NO solution/implementation code** — UNLESS the user explicitly delegates a
  mechanical rung (e.g. "finish the plumbing" → then the assistant implements it).
- **Concepts live in `docs/chain.md`** (living doc, engine-design-first, EDOPro cited
  inline). Extend it as each concept lands.
- **`play.rs` / TUI updates DEFERRED** until the whole chain system is done.
- ADHD-friendly comms (lead with the answer, short bullets, diffs, dry-runs — see
  `CLAUDE.md`). Determinism sacred (BTreeMap, sort-before-iterate, no HashMap). Mirror
  EDOPro (`~/Github/edopro/ocgcore/` + `docs/engine-book/`); cite `file:line`.
- Memory `chain-phase-hands-off-mode` captures this.

## CURRENT — the chain engine (C1/C2, spell speed merged in)
- **C0 done + committed** (`9dc189e`): activation builds a chain
  (`Duel.chain: Vec<ChainLink{effect_seq, card, activator, targets}>`),
  `resolve_chain()` walks **LIFO** (pop back), `Processor::ResolveChain` drives it.
  `activate`/`resume` now **push a link** (don't resolve inline); the driver's
  `Activate` arm pushes `ResolveChain`.
- **Now designing C1** (opponent response window) **+ C2** (chain a 2nd effect →
  LIFO), **with spell speed merged in.** Full spec + dry-runs + EDOPro citations +
  build order in **`docs/chain.md`**.
- **Confirmed vs EDOPro** (in the doc): priority = opponent-of-adder first, ping-pong,
  **two consecutive passes close** (reset on any add); spell speed **derived** from
  `(kind, class, subtype) → 0..3` (truth table in doc); `send` ≠ `destroy`.
- **Build order (docs/chain.md):** 1 plumbing ✅ · **2 spell-speed derive ← NEXT** ·
  3 `send(card, zone)` verb (+ `zones.lua`, a `to_move` list) · 4 chainability gate
  (speed ≥ top link; SS1 never responds) · 5 `ChainResponse` processor (C1 then C2
  ping-pong, two-flag pair on the `Duel`) · 6 fixtures + red tests.
- **`Processor::ChainResponse { step, player }` is stubbed `todo!()`** in driver.rs —
  that's rung 5.

## Rung 1 (plumbing) — DONE this session
- `CardData` gained `spell_type: Option<u32>` + `trap_type: Option<u32>`; `level` is
  now `Option<u32>` (one slot for level/rank/link, EDOPro-style, `None` for
  Spells/Traps). `register_card` harvests all three (Lua nil → `None`).
- Prelude now `include_str!`s `spell_types.lua` **and** `trap_types.lua`
  (`spell_types` had been missing → `SPELL_*` were undefined at card-load).
- `board.rs::level_of` → `and_then` (Spell → `None`); demo panel `unwrap_or(0)`.

## Engine architecture
- `src/duel/`: `mod.rs` (Duel + Lua VM + `load_prelude`), `board.rs`
  (zones/`destroy`/`summon`/`send_to`), `driver.rs` (`step()`/`process()`/`run_unit`,
  processor stack), `scripting.rs` (`activate`/`resume`/`resolve_effect`/
  `resolve_chain`/`process_events`/`activatable_effects`), `battle.rs`. `src/chain.rs`
  (`ChainLink`).
- **Effects are Lua tables** in `effects: Vec<(u32 code, Table)>` (filter by owning
  card code). Metadata via `table.get`: `kind` (ACTIVATE/IGNITION/QUICK/TRIGGER),
  `category` (list of `EFF_CAT_*`), `event` (single `u32`), `optional` (bool).
- **Effect stages** (Lua methods): `condition`/`cost`/`target`/`resolve`. `target`
  runs on a **coroutine** that yields for a selection (freeze → `resume`). Verbs
  record into a shared `EffectContext` scratchpad (`to_destroy`, `targets`,
  `candidates`, `costs`, `activator`) which the Duel applies ("describe, then
  execute").
- **Prelude** `src/duel/prelude/*.lua`: players, effect_kinds, categories,
  card_types, **spell_types, trap_types**, attributes, races, base, events.
- **Events:** `event.rs` `DuelEvent{code,card,reason}`; `EVENT_DESTROYED=1`,
  `EVENT_BATTLE_DESTROYED=2`. `process_events()` (central drain in `step()`) matches
  trigger effects and resolves mandatory inline / freezes optional via
  `MSG_SELECT_YESNO`. **NOTE:** keep triggers resolving inline for now — moving
  triggers onto the chain is **C4** (deferred), and a premature switch broke tests
  earlier this session.
- `activatable_effects` filters by kind + `condition` + `has_legal_target` (runs the
  `target` stage on a scratch coroutine, drops no-candidate effects).

## Fixtures + cards
- **Fixtures** (made-up, tracked) in `tests/fixtures/*.lua`: ExampleSpell (12345678),
  CantActivate (11111111), Avenger (90000001), DudTrigger (90000002), Retaliator
  (90000003), TriCatSpell (90000004), OptionalAvenger (90000005), **Nuke** (90000006,
  no-target destroy-all opponent monsters — the chain-test spell).
- **Real cards** (real passcodes, **untracked** on disk) in `cards/*.lua`: Kuriboh,
  BeaverWarrior, FeralImp, MysticalElf, PotOfGreed, YoureInDanger. The demo loads
  BOTH dirs; tests use `tests/fixtures/…` except `test_card_stats` →
  `cards/BeaverWarrior.lua`.

## Roadmap after the chain
1. **Breadth:** `EVENT_SUMMONED` + verbs (`draw`, `damage`) so real cards work.
2. **"When" / miss-the-timing** refinement (point-event vs full-event).
3. Revisit `play.rs` for the chain UI (deferred until chain is done).
