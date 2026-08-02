# Modifiers & Continuous Effects

**What this chapter covers:** how a lasting effect (a +500 buff, "ATK becomes 0", "you take no battle damage") is stored as a **modifier** and folded into an answer at query time — not applied once and forgotten.

**Mental model:** a modifier is a **standing note taped to a card (or a player)**. Nothing changes the printed stat. When someone *asks* "what's this ATK?", the engine reads the printed value and folds the notes in, right then.

---

## The model: standing conditions, consulted at query points

- A **one-shot action** happens once: "destroy that card." Done, gone.
- A **continuous effect** is a *standing condition*: "while this is on the field, all your monsters gain 500 ATK." It has no single moment — it must be true **every time you look**.
- cardcrusher models the second kind as a **`Modifier`** that lives on the card (or player) and gets **folded in at the query point** (`atk_of`, `def_of`, `can_take_battle_damage`).

> The printed stat is never mutated. `atk_of` recomputes from scratch every call.

---

## `Modifier` — the standing note

`src/modifiers.rs:8`:

```rust
pub struct Modifier {
    pub id: u32,          // unique — lets you remove THIS one
    pub source: CardId,   // the card that produced it — remove ALL of a source's notes
    pub mod_type: ModifierType,  // WHAT it changes
}
```

- **`id`** = a unique handle. Remove exactly one note (EDOPro's `effect::id`).
- **`source`** = the producing card. Remove *every* note a card made (EDOPro's `effect::owner`) — e.g. when that card leaves the field.

The kinds (`src/modifiers.rs:16`):

| `ModifierType` | Effect | In the fold? |
|----------------|--------|--------------|
| `AtkChange(i32)` | ±N to ATK | yes (ATK) |
| `DefChange(i32)` | ±N to DEF | yes (DEF) |
| `SetAtk(i32)` | ATK **becomes** N | yes (ATK) |
| `SetDef(i32)` | DEF **becomes** N | yes (DEF) |
| `NoBattleDamage` | "you take no battle damage" | no — a boolean gate |

Lua names them with `MOD_*` codes (`src/duel/prelude/modifiers.lua`), bridged by `ModifierType::from_code` (`src/modifiers.rs:43`): `0→AtkChange`, `1→DefChange`, `2→SetAtk`, `3→SetDef`, `4→NoBattleDamage`.

---

## The priority fold in `atk_of` / `def_of`

**The answer:** gather the card's ATK notes, **stable-sort by priority**, apply in that order, floor at 0. Priority makes `SetAtk` (the base) land *before* `AtkChange` (the stack) no matter what order they were added.

Priorities (`src/modifiers.rs:29`) — **lower applies first**:

| `ModifierType` | `priority()` | Band |
|----------------|-------------|------|
| `SetAtk` / `SetDef` | `Some(0)` | base — "becomes X" |
| `AtkChange` / `DefChange` | `Some(1)` | stack — "+N on top" |
| `NoBattleDamage` | `None` | not in the value fold |

The fold (`src/duel/board.rs:47`):

```rust
pub fn atk_of(&self, card: CardId) -> Option<i32> {
    let card = self.cards.get(card)?;
    let mut mods: Vec<ModifierType> = card.modifiers.iter()
        .map(|m| m.mod_type)
        .filter(|mt| matches!(mt, AtkChange(_) | SetAtk(_)))
        .collect();
    mods.sort_by_key(|mt| mt.priority());   // STABLE — ties keep insertion order

    let mut atk = card.data.atk;             // printed value
    for mt in mods {
        match mt {
            SetAtk(n) => atk = n,            // base band: replace
            AtkChange(n) => atk += n,        // stack band: add
            _ => {}
        }
    }
    Some(atk.max(0))                         // floor at 0
}
```

`def_of` (`src/duel/board.rs:72`) is the exact mirror with `SetDef`/`DefChange`.

### ASCII: the fold pipeline

```
 card.data.atk (printed)
        │
        ▼
 gather ATK notes ─────────────────────────┐
   [ AtkChange(+500), SetAtk(0) ]           │  (as inserted)
        │                                    │
        ▼  stable-sort by priority()         │
   [ SetAtk(0){p=0}, AtkChange(+500){p=1} ]  │
        │                                    │
        ▼  apply left→right                  │
   atk = 0        (SetAtk sets base)         │
   atk = 0 + 500  (AtkChange stacks)         │
        │                                    │
        ▼  .max(0)  (floor)                  │
      500  ◄───────────────────────────────┘
```

### Dry-runs

- **Stacking** (`test_modifiers.rs:56`): base 1000, `AtkChange(+300)`, `AtkChange(-100)` → `1000 + 300 − 100 = 1200`.
- **Floor** (`:83`): base 1000, `AtkChange(-1500)` → `-500` floored → **0**.
- **Priority beats insertion** (`:133`): base 1000, add `AtkChange(+500)` **first**, then `SetAtk(0)`:
  - naive insertion order would give `1000 + 500 = 1500 → set 0` = **0** (wrong).
  - sort by priority → `[SetAtk(0), AtkChange(+500)]` → `0 + 500` = **500** (correct).
  - The sort is why insertion order between a SET and an ADD doesn't matter.

---

## Player modifiers: the `NoBattleDamage` gate

Some conditions belong to a **player**, not a card. Kuriboh's "you take no battle damage" fires on a **direct attack** — there's no defending monster to tape the note to. So the note goes on the player.

Storage (`src/duel/mod.rs`): `player_modifiers: [Vec<Modifier>; 2]` — one list per player, parallel to the per-card lists.

`NoBattleDamage` is a **boolean gate**, not a value fold. It has `priority() == None` and is read by `can_take_battle_damage` (`src/duel/board.rs:417`):

```rust
pub fn can_take_battle_damage(&self, player: usize) -> bool {
    !self.player_modifiers[player]
        .iter()
        .any(|m| m.mod_type == ModifierType::NoBattleDamage)
}
```

The **damage step honors it** (Chapter 7, step 2, `src/duel/driver.rs:373`): after computing `pending_damage`, it zeroes any protected player's share.

```
preview:  [0, 1500]        (P0 attacks P1 directly for 1500)
gate P1:  can_take_battle_damage(1) == false   → dmg[1] = 0
apply:    [0, 0]           → P1 stays at 8000
```

That's exactly `test_no_battle_damage.rs:55`: P1 with a `NoBattleDamage` player modifier ends the direct attack at **8000**.

---

## Adding & removing

Add hooks (`src/duel/board.rs`):

| Function | Adds to | Returns |
|----------|---------|---------|
| `add_modifier(card, source, mod_type)` (`:363`) | a card's list | new `id` |
| `add_player_modifier(player, source, mod_type)` (`:400`) | a player's list | new `id` |

Both draw the `id` from `next_modifier_id()` (`src/duel/board.rs:355`) — a counter that lives in `ctx`, shared so a Lua verb and the engine hand out from the **same sequence** (Chapter 9 explains why that matters).

Remove hooks — **two granularities**:

| Function | Removes |
|----------|---------|
| `remove_modifier(id)` (`src/duel/board.rs:389`) | the **one** note with that id — from any card or either player list |
| `remove_modifiers_from(source)` (`:378`) | **every** note that `source` produced — across all cards *and* both player lists |

```rust
// remove_modifier: scan everywhere, drop the single matching id
pub fn remove_modifier(&mut self, id: u32) {
    for (_, card) in self.cards.iter_mut() {
        card.modifiers.retain(|m| m.id != id);
    }
    for mods in self.player_modifiers.iter_mut() {
        mods.retain(|m| m.id != id);
    }
}
```

Dry-runs:

- **By id** (`test_modifier_expiry.rs`): a queued `remove_modifier(passive)` drops just that one note when the battle ends → protection gone, everything else intact.
- **By source** (`test_modifiers.rs:152`): card has `+500` from A and `+300` from B → `remove_modifiers_from(A)` → only `+300` remains → ATK `1300`.
- **Source spans cards** (`:169`): one source buffs X and Y → `remove_modifiers_from(source)` clears **both**.

---

## In one breath

- A **`Modifier { id, source, mod_type }`** is a standing note on a card or a player — the printed stat is never touched.
- `atk_of`/`def_of` **fold** the notes at query time: stable-sort by priority (`SetAtk` base band before `AtkChange` stack band), apply, floor at 0.
- **`NoBattleDamage`** is a player-scoped boolean gate (`can_take_battle_damage`) — player-scoped because Kuriboh works on direct attacks with no monster to attach to.
- Remove **one** note by `id` (`remove_modifier`) or **all of a source's** notes (`remove_modifiers_from`).
