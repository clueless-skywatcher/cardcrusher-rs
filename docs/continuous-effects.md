# Continuous & replacement effects — the generic model

**Living doc.** The *why* and the *shape* of a generic continuous-effect system —
engine design first, Yu-Gi-Oh! rules + EDOPro cited where they explain a choice.
No implementation code here; this is the map. Written because the rung-5
lingering-effect code is a **narrow special case** (battle-damage only) and needs
a real model before it grows.

## The one idea

A continuous effect **does not run**. It's a **standing modifier the engine
consults** at the moment a quantity is read. To be generic it needs four things:

```
continuous effect = { quantity,   value,        range,      lifetime }
                       ^what it    ^a FUNCTION    ^who/what   ^when it
                        changes     (any logic)   it hits     turns off
```

"Generic" is **not** "an unlabeled closure". The closure stays (that's the
*value*), but it must say **which quantity** it changes, so the engine knows
**where to consult it**.

## Why unlabeled closures don't scale

Today `run_lingering_effects()` runs *every* stored closure at *one* moment
(damage). Add a second kind and it falls over:

```
store = [ fn:"zero my battle damage",  fn:"my monster gets +500 ATK" ]

atk_of(monster)        → ??? we don't run closures here, so +500 never applies
apply battle damage    → runs BOTH; the +500-ATK fn pokes nothing, or the wrong thing
```

With no label, every query point would have to run every closure and hope each
mutates the right variable. That's not generic — it's undefined behaviour. **The
label (the query point) is what makes it composable.**

## Three machines — do NOT conflate them

The examples that motivated this (ATK/DEF, double attack, GY redirect, Maxx "C")
are **three different mechanisms**:

| Effect | Machine | Consulted at… |
|---|---|---|
| ATK/DEF change | **continuous modifier** | `atk_of` / `def_of` |
| Extra attack / turn | **continuous modifier** | the "attacks allowed?" query |
| GY → deck redirect | **replacement** (intercept + swap) | the `send_to` chokepoint |
| Maxx "C" (draw on each SS) | **triggered subscription** | on `EVENT_SPECIAL_SUMMON` |

Our rung-5 battle-damage immunity is Machine 1 with exactly **one** query point
wired and **no** labels.

---

## Machine 1 — continuous modifiers (query-fold)

**Shape:** at each readable quantity, gather the effects tagged for it, fold them
in a defined order, return the result.

- **Quantity** = a generic slot, an *open set*: `ATK`, `DEF`, `ATTACK_COUNT`,
  `CAN_TAKE_BATTLE_DAMAGE`, … Add new ones as cards need them.
- **Value** = a function `(current, ctx) -> new`. Keeps it function-first: the
  logic can read game state (e.g. "+100 ATK per card in my GY").
- **Range** = which cards/players it applies to.
- **Fold order matters** — "set to 0" then "double" ≠ "double" then "set to 0".
  EDOPro sorts by effect id before folding; we must **sort before iterate**
  (determinism, see `src/lib.rs`).

**Dry run — ATK:**
```
atk_of(card):
    base = printed ATK
    for each ATK-modifier in force, in sorted order:
        base = modifier.value(base, ctx)
    return base
```
Battle-damage immunity is the same shape at a boolean quantity:
`can_take_battle_damage(player)` folds to false if any immunity is in force.

**EDOPro:** `card::get_attack()` gathers `EFFECT_UPDATE_ATTACK` (add) and
`EFFECT_SET_ATTACK` / `EFFECT_SET_ATTACK_FINAL` (overwrite) via `filter_effect`,
sorted, then applies. Extra attacks = `EFFECT_EXTRA_ATTACK` /
`EFFECT_EXTRA_ATTACK_MONSTER`. Battle-damage immunity =
`EFFECT_AVOID_BATTLE_DAMAGE`, checked in `calculate_battle_damage`
(`processor.cpp`). Each is a **query point**, each value can be a Lua function.

---

## Machine 2 — replacements (intercept + swap)

**Shape:** at an action's **chokepoint**, ask "does a replacement want to change
what happens?" If yes, do the replacement instead of the default.

- Not a value fold — an **interception**. "If a card *would* go to the GY, send it
  to the Deck instead." "If this *would* be destroyed, banish it instead."
- Usually **optional** ("you can") and **once per resolution** — needs care so it
  doesn't loop.
- Chokepoints we already have: `send_to`, `destroy`. That's exactly where a
  redirect hooks in.

**Dry run — GY redirect:**
```
send_to(card, GY):
    if a redirect effect applies to (card, GY):
        destination = redirect.value(card)   -- e.g. Deck
    else:
        destination = GY
    ...move card to destination
```

**EDOPro:** destination redirects = `EFFECT_TO_GRAVE_REDIRECT`,
`EFFECT_TO_HAND_REDIRECT`, `EFFECT_TO_DECK_REDIRECT`; battle-destroy redirect =
`EFFECT_BATTLE_DESTROY_REDIRECT` (seen in the damage step, `processor.cpp` ~2928).
Full "instead of" replacements run through the `OperationReplace` processor with
`EFFECT_DESTROY_REPLACE` / `EFFECT_SEND_REPLACE` and a check/operation function.

---

## Machine 3 — triggered subscriptions (Maxx "C")

**Not continuous at all.** Maxx "C" = "for the rest of THIS turn, **each time** the
opponent Special Summons, draw 1." That's an **event subscription with a
lifetime**, not a standing modifier.

- **Event** it listens for (`EVENT_SPECIAL_SUMMON`), a **count** (unlimited this
  turn), a **lifetime** (until the turn ends), a **callback** (draw 1).
- This is **rung 6's `queue(event, {count, period}, fn)`** — the same machine that
  removes Kuriboh's immunity at `EVENT_BATTLE_ENDED`. See `docs/kuriboh.md`.

**EDOPro:** Maxx "C" registers a delayed effect reacting to the special-summon
event with a per-turn count and a turn-scoped reset — the trigger/subscription
path, distinct from `filter_effect` value folds.

---

## EDOPro paradigm — do we follow it?

**Target model: yes.** Ours will map onto EDOPro's:

| Ours (target) | EDOPro |
|---|---|
| quantity (query-point label) | the `EFFECT_*` code |
| value = function | effect value (constant or Lua function) |
| range | `effect->range` / `s_range` / `o_range` |
| lifetime | `RESET_*` flags + reset event |
| Machine 1 fold | `filter_effect` + sorted apply |
| Machine 1 boolean | `is_affected_by_effect` |
| Machine 2 replacement | redirect effects + `OperationReplace` |
| Machine 3 subscription | delayed/continuous **triggered** effect |

**Current code: no** — rung 5 is Machine 1 with one hard-coded query point
(damage) and no labels. It works for Kuriboh; it doesn't generalise.

---

## Retrofit plan (rung-5 code → the model)

1. **Tag lingering effects with a quantity**, not just a bare closure:
   `add_lingering_effect(quantity, valueFn)`. Keeps the function; adds the label.
   (This is *not* the "constant-named behaviour" we rejected — the label is a
   generic query-point, the behaviour still lives in the function.)
2. **Add query points** as folds: `atk_of` / `def_of` gain a modifier fold;
   introduce `attacks_allowed(monster)` and `can_take_battle_damage(player)`.
   Battle damage stops being special — it's just the `CAN_TAKE_BATTLE_DAMAGE`
   query.
3. **Replace `run_lingering_effects`** (blind fire-all) with per-query gather-and-
   fold, **sorted by handle id** for determinism.
4. **Replacements + subscriptions are separate builds** — don't force them through
   the modifier store.

## Lifetime & determinism (applies to all three)

- **Handle** — every registered effect gets a stable, never-reused id (already
  true). Removal (`reset_passive`) is by handle.
- **Lifetime** — "that battle" / "this turn" / "while face-up on the field". Until
  we model ranges, removal is via a `queue(...)` subscription (rung 6).
- **Determinism** — sorted `Vec`/`BTreeMap`, **sort before iterate**, never a
  `HashMap`. Fold order is part of the ruling, not an accident.

## Open questions

- **Where do query points live?** One registry of folds, or a method per quantity?
- **Function value re-evaluation** — a fold runs the value fn every read. Fine for
  determinism (no cached staleness), but watch cost.
- **Replacement loops** — a redirect that itself would be redirected. EDOPro guards
  this; we'll need to.
- **Range without a range model** — until effects know "while face-up on the
  field", everything leans on explicit `reset_passive`. Acceptable near-term.
