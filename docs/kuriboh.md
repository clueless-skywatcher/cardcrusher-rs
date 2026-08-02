# Kuriboh — lingering effects & subscriptions (rungs 5–7)

**Living doc.** The *why* and the *shape* of the remaining Kuriboh rungs — engine
design first, Yu-Gi-Oh! rules + EDOPro cited where they explain a choice. No
implementation code here (that's the codebase); this is the map.

Kuriboh is a **feature epic**: one card that forces five separate mechanisms.
Rungs 1–4 (self-card, discard cost, damage-calc window, quick-from-hand) are
**built**. This doc covers what's left.

## The card, and where each stage lands

```lua
function nullify_bd:condition(e)
    return e:in_hand(YOU)                     -- rung 1 ✅
       and e:current_player() == OPPONENT     -- rung 6 (current_player verb)
       and e:battle_damage() > 0              -- rung 3 ✅
end
function nullify_bd:cost(e) e:discard_self() end   -- rung 2 ✅
function nullify_bd:resolve(e)
    local h = e:add_lingering_effect(function() e:disable_battle_damage() end)  -- rungs 5 + 7
    e:queue(EVENT_BATTLE_ENDED, { 1, ONCE }, function() e:reset_passive(h) end) -- rung 6
end
```

## Status

| Rung | Concept | State |
|---|---|---|
| 1 | self-card (`in_hand`) | ✅ done |
| 2 | discard cost (`discard_self`) | ✅ done |
| 3 | damage-calc window + pending damage (`battle_damage`) | ✅ done |
| 4 | quick-from-hand offered at the window | ✅ done |
| **5** | **lingering effect** — store a generic closure + **honor it at damage** (rung 7 folded in) | ✅ done |
| **6** | **subscription/queue** + activate-at-window → chain → resolve | 🔜 next |
| ~~7~~ | ~~honor the effect~~ — done as part of rung 5 for the battle-damage case | ✅ folded in |

> **Scope note:** rung 5 originally meant "store only". But a stored closure that
> nothing honors *causes no effect* — so honoring (old rung 7) came with it. A
> stored lingering effect is meaningless in isolation; the two are one unit.

---

## Rung 5 — lingering effects *(next)*

### What it is
A **continuous effect** that outlives the stage that made it. Kuriboh's resolve
registers "you take no battle damage from that battle" and it must still be there
when damage is calculated later. `add_lingering_effect(fn)` takes a **function**
(deliberately generic — it can do anything) and returns a **handle** so it can be
found and removed later.

### The one design question: where does it live?
Not in `EffectContext` — that's a per-stage scratchpad, wiped between stages. A
lingering effect survives across stages and turns → **new persistent state on
`Duel`**.

Sharp edge: it holds a **Lua function**. Rust can't casually keep a Lua value
around → use **mlua's registry** (`create_registry_value` / `registry_value`). A
stored lingering effect = *a registry key + a handle id*, **not** the raw
function.

Determinism (see `src/lib.rs`): a **sorted `Vec`/`BTreeMap` keyed by an
incrementing id**, never a `HashMap`.

### Where to change what
- **`src/duel/mod.rs`** — the store's home. New `Duel` field: the list of active
  lingering effects + a "next handle id" counter. Create it in `new()`. The verb
  must write to it, so wrap it `Rc<RefCell<…>>` (like `effects`/`field`) and thread
  it through `set_globals` → `register_verbs`.
- **`src/effect.rs`** — the verb `effect_add_lingering_effect(fn)`: stash `fn` in
  the registry, push `{handle, key}` into the store, **return the handle** to Lua.
  Decide where the little stored-effect struct type lives (here, or a tiny module).
- **`src/duel/prelude/base.lua`** — one wrapper line: `Effect:add_lingering_effect`.
- **an accessor** (`scripting.rs`) — something observable like `lingering_count()`
  for the test.

### What got built (scope grew past "store only")
- `add_lingering_effect(fn)` stores a closure (in the activator's frame) + returns a handle.
- `disable_battle_damage()` zeroes the activator's `pending_damage`.
- `run_lingering_effects()` fires them all; the `Attack` damage step calls it between
  `battle_damage_preview` and applying, so the standing condition is honored.
- `resolve_battle` split into `apply_battle_damage` + `apply_battle_destruction`
  (calc → apply → destroy, EDOPro steps 26/27/28) so the processor path applies the
  *lingering-modified* damage while direct callers keep the all-in-one form.

Test: P1 registers "no battle damage" → P0 attacks P1 for 1500 → P1 stays 8000.

Still not done (→ rung 6): **removing** one (`reset_passive` — the handle already
suffices to find-and-remove) and **scoping** ("that battle" needs the
`queue(EVENT_BATTLE_ENDED, …)` subscription to remove it; today it lingers forever).

### Reference — why we diverge
EDOPro's continuous effects are **typed**: `EFFECT_AVOID_BATTLE_DAMAGE` added via
`card::add_effect` with `EFFECT_TYPE_FIELD` (`effect.h`, and the damage check in
`processor.cpp` `calculate_battle_damage`). We go **generic** — a stored closure,
not a typed effect — per the "keep the API generic" call. Intentional
simplification: our engine has no effect-type taxonomy yet, and a closure covers
the one behavior we need.

---

## Rung 6 — subscriptions (`queue`) + resolve-at-window *(sketch — firm up when we start)*

### Two things bundled here
1. **`queue(event, {count, period}, fn)`** — fire `fn` on a future `event`, up to
   `count` times within `period`. Kuriboh uses `{ 1, ONCE }` to remove its
   lingering effect when the battle ends. Shares the `{count, period}` frequency
   model with `effect.frequency` (activation limits).
   - `count`: `1` / `N` / `INFINITE`. `period`: `PER_TURN` / `THIS_TURN` /
     `PER_BATTLE` / `ONCE`.
   - Needs: `reset_passive(handle)` (remove a rung-5 lingering effect by handle),
     `current_player()` (whose turn it is, relative to the activator), and the
     event-queue plumbing to actually deliver `EVENT_BATTLE_ENDED`.
2. **Activate-at-window → chain → resolve.** The damage window already *offers*
   the quick effect (rung 4). Rung 6 makes choosing it actually **chain and
   resolve**. The seams are already placed: `resolve_chain()` at `Attack` steps 2
   and 3 (`driver.rs`) resolve whatever was built at the before/after-damage
   windows. Verify the flow: activate → `push_chain_link` → window ping-pong →
   `resolve_chain` → the lingering effect gets registered.

### Where to change what (provisional)
- **`src/effect.rs`** — verbs `queue`, `reset_passive`, `current_player`.
- **`src/duel/mod.rs`** — a subscription store (like rung 5's), plus the
  `{count, period}` bookkeeping (decrement/expire).
- **event delivery** — wherever `EVENT_BATTLE_ENDED` is raised, drain matching
  subscriptions. (Find where battle end is signalled today; may need the event
  added.)

---

## Rung 7 — honor the effect *(sketch)*

### What it is
The lingering effect must actually **do** something: `disable_battle_damage`
should make the pending battle damage **zero**. Today `resolve_battle` /
`deal_damage` apply damage blindly.

### Where to change what (provisional)
- **`src/duel/battle.rs`** — before applying, check whether the damaged player is
  covered by an active "avoid battle damage" lingering effect; if so, zero it.
- The **mechanism**: `disable_battle_damage` (called *inside* the stored closure at
  damage time) sets a flag / writes into `pending_damage`. Decide: does the closure
  run eagerly (mutating `pending_damage` now) or is it a predicate checked at apply
  time? EDOPro checks at calculation time (`is_player_affected_by_effect(pd,
  EFFECT_AVOID_BATTLE_DAMAGE)` → `core.battle_damage[pd] = 0`, `processor.cpp`
  `calculate_battle_damage`). Mirror that: **check at apply time.**

### Reference
`calculate_battle_damage` (`processor.cpp` ~2929) zeroes `core.battle_damage[pd]`
when the player is affected. Our `pending_damage` is the analog of
`core.battle_damage`; the "avoid" lingering effect is the analog of the player
being affected. Same shape, generic closure instead of a typed effect.

---

## Open questions to answer as we build
- **Lifetime of a lingering effect** — Kuriboh's lasts "that battle". Is removal
  purely via the `queue(EVENT_BATTLE_ENDED)` subscription, or does the engine also
  need a general expiry pass? (Start with subscription-only; revisit.)
- **Handle stability** — the handle returned by rung 5 must stay valid until rung 6
  removes it. Incrementing ids never reused within a duel.
- **Registry cleanup** — when a lingering effect / subscription is removed, free its
  registry value so the Lua side doesn't leak (GC is stopped for determinism).
