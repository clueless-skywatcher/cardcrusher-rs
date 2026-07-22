# cardcrusher vs ygopro-core

Our Rust engine vs EDOPro's C++ engine. **Only the parts built so far.**

Citations: `src/…` = us · `file.cpp:NN` = ygopro-core.

---

## Scorecard

| Subsystem | Match? | The difference |
|-----------|:---:|----------------|
| Processor / driver | ✅✅ | basically a port |
| Freeze / resume | ✅✅ | identical |
| Turns & phases | ✅ | we push a new Turn each time; 7 steps not 20 |
| Main-Phase menu | ✅ | 1 unit not 2; 1 byte not 16-bit |
| Draw / move / LP | ✅ | we skip the events/effects |
| Object storage | ✅ | **we're safer** (generational keys) |
| Determinism | ✅ | **we're safer** (ordered maps) |
| Scripting | ⚠️ | **no coroutine bridge** ← the big gap |
| Ownership | ❌ | inverted (see §8) |

---

## 1. Storage & deletion

- **EDOPro:** `unordered_set` of heap objects, referred to by **raw pointers**. On delete, repoints the Lua handle to a shared `DELETED` sentinel so stale refs don't dangle (`interpreter.cpp:162`).
- **Us:** `SlotMap<CardId, Card>`, referred to by **generational keys** (`src/ids.rs`). Stale key → `None`, automatically.
- **Diff:** their safety is hand-built; ours is free and can't crash.

## 2. Field & zones

- **EDOPro:** location = **bitflag byte** + `loc_info{controler,location,sequence,position}`; per-player `vector<card*>` piles (`field.h:82`).
- **Us:** `Zone` enum + `BTreeMap<CardId,Zone>` + `decks`/`hands: [Vec;2]` (`src/field.rs`).
- **Diff:** we lack controler/sequence/position and zone slots.

## 3. Processor & driver ⭐ (most alike)

- **EDOPro:** units derive `Process<needs_answer>` with a `step`; ~80 in a `variant`; **33-line driver** `field::process()` (`processor_visit.cpp:9`).
- **Us:** `enum Processor`, `Vec` stack, `step()` driver (`src/duel.rs`).

| | EDOPro | Us |
|---|---|---|
| children run first | `splice(subunits, front)` | `insert(depth_before, unit)` |
| who bumps `step` | the driver | the handler |
| `needs_answer` | compile-time bool | runtime `match` |
| one-step vs loop | `process()` / `OCG_DuelProcess` | `step()` / `process()` |

## 4. Freeze / resume

- Both: return **`Awaiting`**, leave the unit on the queue, host writes a response buffer, next tick re-runs the unit's later step.
- **Diff:** none. (We don't validate/`MSG_RETRY` yet.)

## 5. Turns & phases

- **EDOPro:** one `Turn`, ~20 steps, skips + 2nd Battle Phase. Handover = `step=restart` + flip player → **reuses one unit** (`processor.cpp:3614`).
- **Us:** one `Turn{step,player}`, **7-entry phase table**; handover = **push a fresh Turn** (bounded by `max_turns`).
- **Diff:** no skips/events; turn-count guard instead of real game-end.

## 6. Main-Phase menu

- **EDOPro:** 2 units — `IdleCommand` (builds options) + `SelectIdleCmd` (asks). Response = `int32`: **type = low 16, index = high 16** (`playerop.cpp:142`).
- **Us:** 1 `IdleCommand`; response = **1 byte** (`0` = next phase, else re-show menu).
- **Diff:** we fuse the two, no options computed yet. (Our "re-show" = their shuffle-uses-`restart` trick.)

## 7. Draw / move / LP

| | EDOPro | Us |
|---|---|---|
| draw | `list_main.pop_back()` → hand; `overdraw` flag | `decks[p].pop()` → hand; `None` |
| destroy | `move_card` + events | `send_to(id, Zone::GY)` |
| LP | `int32 lp`, via `Damage`/`PayLPCost` units + events | `[u32;2]`, `saturating_sub` |

- **Diff:** they route every change through events/effects (so cards react). We mutate directly. That's Phase 3.

## 8. Scripting ⚠️ (the big gap)

- **EDOPro:** effect stores **4 callable refs** (`condition/cost/target/operation`, `0`=none, `effect.h:50`). **Coroutine bridge** (`interpreter.cpp:571`): an operation runs on its own Lua thread and **pauses mid-line** (`yieldk`), resuming with the answer.
- **Us:** effect stores **1 `resolve` closure** + `has_target` (`src/script.rs`). No coroutines — `CardLibrary` drives freeze/resume *around* the closure: freeze for the target *first*, then run `resolve` start-to-finish.
- **Diff:**
  - their script asks the player *at any point*; ours must ask **before** `resolve` runs.
  - `cost`/`target` in our DSL are still no-op placeholders.
  - **Ownership inverted:** EDOPro — duel *owns* the interpreter (`duel*` in Lua extraspace). Us — `CardLibrary` holds a shared `Rc<RefCell<Duel>>`.

## 9. Determinism

- **EDOPro:** hash containers, but **sorts before** order-sensitive steps; integer-only; no GC.
- **Us:** **ordered containers everywhere** (no sorting needed); integer-only.
- **Diff:** same guarantee, fewer footguns.

---

## Verdict

**Engine core = faithful port.** Two wins: safer memory (generational keys) and
determinism (ordered maps). Two simplifications: 1 closure vs 4 effect refs, and
**no coroutine bridge** (scripts can't pause mid-run yet). That bridge + the
event/effect machinery = what Phase 3 is about.
