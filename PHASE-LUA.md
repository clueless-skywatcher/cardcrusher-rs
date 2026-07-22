# Phase Lua — the scripting layer, from scratch

The card DSL is built on **Lua** (via `mlua`). Lua has real coroutines, so a card
can pause mid-effect to ask the player a question, then resume — the thing our
old Rhai prototype couldn't do.

**Clean slate:** all Rhai code is gone. `src/script.rs` is an empty room. The
crate compiles and every non-scripting test is green. We build the scripting
layer back test-first, from nothing.

Background: **[LUA-PRIMER.md](LUA-PRIMER.md)** (the language) and
**[MLUA-GUIDE.md](MLUA-GUIDE.md)** (the Rust bridge).

> **How to read this:** each milestone starts by **writing a small failing test**
> (Claude's job, per our TDD split), then you implement until it's green. One
> milestone per test. Small steps.

---

## What a card looks like (the target API)

```lua
Example = Card:new(12345678)          -- a card is an object

local activate = Example:add_effect()  -- a card HAS effects; add each one

function activate:cost(e)    e:pay_lp(500) end
function activate:target(e)  e:prompt_selection(e:monster_zone(OPPONENT), 1) end
function activate:resolve(e) e:destroy(e:targets()) end
```

- **`Card` / `Example`** — the static definition (a class; inherits defaults).
- **`add_effect()`** — a card can hold many effects.
- **`e`** — the effect at runtime. Verbs are **methods on `e`** (no globals).

---

## The map

```
Prelude + load ─► Register ─► Resolve ─► Cost ─► COROUTINE ─► Targeting ─► Menu
     (M1)          (M1)        (M2)      (M3)     (M4 ⭐)       (M5)        (M6)
```

Vocabulary to build (all as methods on the `e` effect object, plus a small
`Card` base class in Lua): `add_effect`, `pay_lp`, `destroy`, `targets`,
`prompt_selection`, `monster_zone`/`monsters`, and the constants `YOU`/`OPPONENT`
+ the zone values.

---

## M1 — VM + prelude + load a card

**Do:** give `Duel` a Lua VM (`Lua::new()`, then **`gc_stop()`** — determinism,
MLUA-GUIDE §8). Inject the **`Card` base class** (a small Lua prelude: `new`,
`add_effect`, and default effect stages). Add `load_card(path)`: read the file,
`load(src).exec()`, then the card's `Card:new`/`add_effect` calls register its
effects. Expose an effect count so a test can check.

**Why:** proves the embedding — Lua loads, the prelude works, effects register.

**Done when:** a new `tests/test_lua_scripting.rs` loads `cards/Example.lua` and
sees exactly 1 effect.

---

## M2 — Resolve applies to the board

**Do:** expose `e` as an object (an mlua `UserData`, MLUA-GUIDE §7 — or a table
backed by a shared context). Give it `e:destroy(...)` and `e:targets()`. Running
an effect's `resolve` sends the target to the GY. Keep "describe, then execute"
if you route through a shared context.

**Done when:** a test resolves the effect and the target lands in `Zone::GY`.

---

## M3 — Cost is paid on activation

**Do:** add `e:pay_lp(n)`. Activating runs `cost` before `resolve`; LP drops.

**Done when:** a test activates and sees the activating player's LP fall by 500.

---

## M4 — ⭐ The coroutine bridge (freeze mid-effect, resume with the answer)

**The payoff.** Run the effect on its own Lua **`Thread`** (MLUA-GUIDE §6).
`e:prompt_selection(...)` calls `coroutine.yield` — the processor freezes
(`DuelStatus::Awaiting`). Store the `Thread` on the paused processor unit. When
the host answers, `thread.resume(answer)` continues the *same* Lua function from
the yield, and `resolve` runs with the choice in hand.

**Why:** the whole reason for Lua. `target`/`resolve` pause linearly and resume —
no re-run hack. Lua's `yield`/`resume` and the engine's freeze/resume become one
pause.

**Done when:** a test activates an effect, sees the duel freeze at selection
(nothing destroyed yet), answers, resumes, and the target is destroyed.

---

## M5 — Real targeting: candidate set + relative players

**Do:** `e:monster_zone(who)` / `e:monsters(who, zone)` query the live field for a
player's cards. `who` is **relative** — `OPPONENT` resolves against the activating
player. The yield sends candidates out; the host picks **by index** (Lua is
1-based — mind the off-by-one). Empty candidate set → activation is rejected up
front (cost NOT paid).

**Done when:** tests cover: destroys the chosen opponent monster; `OPPONENT` is
relative to the activator; no legal target rejects the activation.

---

## M6 — Activate from the Main-Phase menu

**Do:** re-add the activation command to the menu (a `CMD_ACTIVATE` + an
`Activate` processor unit were removed in the wipe — rebuild them to drive the
coroutine thread). Wire the menu response to the yield/resume protocol.

**Done when:** a test picks "activate effect" from the menu and the effect
resolves end-to-end.

---

## Cleanup

- `cargo test`, `cargo clippy`, `cargo fmt --check` — all green.
- Commit.

---

## Scoreboard

| M | Behaviour | Status |
|---|-----------|:---:|
| 1 | load a card → 1 effect registered | ✅ |
| 2 | resolve → target to GY | ⬜ |
| 3 | activate → cost paid | ⬜ |
| 4 | freeze at selection → resume → destroy | ⬜ |
| 5 | candidate set, relative players, no-target reject | ⬜ |
| 6 | activate from the menu | ⬜ |
