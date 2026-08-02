# SEGOC — Simultaneous Effects Go On Chain

**The problem:** several triggers fire at the *same instant* (e.g. a board-wipe
destroys three monsters, each with a "when destroyed" effect). They can't all be
"first". SEGOC is the rule that puts them on **one chain in a defined order** — so
the game stays deterministic.

Companion to [`chain.md`](chain.md) (the chain engine); this is milestone **C5**.

---

## The placement order (the whole rule)

When multiple triggers go off together, they're **placed on the chain** in this
order — EDOPro-confirmed against `Processors::PointEvent`:

| # | bucket | placed | resolves (LIFO) |
|---|---|---|---|
| 1 | **turn player · mandatory** | first (chain link 1) | **last** |
| 2 | **opponent · mandatory** | | |
| 3 | **turn player · optional** | | |
| 4 | **opponent · optional** | last (top link) | **first** |

Two rules do all the work:
- **Turn player before opponent.** (`current_player = turn_player`, then flips to
  `1 - turn_player` — `processor.cpp:604`, `:665`.)
- **All mandatory before any optional.** The forced phase fully drains for *both*
  players before the optional phase starts — it's global, not per-player
  (`processor.cpp:664-668`).

> **Chain link 1 resolves LAST.** Resolution is LIFO (see `chain.md`), so the order
> above **reverses** on resolution: the opponent's optional resolves first, the
> turn player's mandatory resolves last.

### Within one player's batch
If one player has several simultaneous triggers, **they choose the order** among
them (`SelectChain`, `processor.cpp:657`). Under the TCG rule
(`DUEL_TCG_SEGOC_FIRSTTRIGGER`, `:646-651`) the choice is narrowed to the triggers
from the *earliest* event first (sorted by `event_id`). Our engine simplifies this
to **event order** (the order the events were raised).

---

## Dry run

Turn player **TP** controls monster A (mandatory trigger); opponent **OPP**
controls monster B (mandatory trigger). A board-wipe destroys both at once.

```
events raised:      [B destroyed, A destroyed]   (say OPP's monster died first)

SEGOC sort:         turn player first, so:
  chain link 1  =   A  (TP mandatory)
  chain link 2  =   B  (OPP mandatory)

resolve LIFO (top→bottom):
  link 2 (B, OPP)  resolves FIRST
  link 1 (A, TP)   resolves LAST
```

Note the event order (`B, A`) is the **reverse** of the placement order (`A, B`) —
SEGOC re-sorts by player, it does **not** follow the order things were destroyed.

---

## How our engine does it

`process_events` is our `process_instant_event` + `PointEvent`. It **collects** all
fired triggers, **sorts** them into SEGOC order, then **builds** the links —
handing off to the chain machinery (`ResolveChain` + `ChainResponse`) we already
have from C0–C4.

```text
process_events():
    turn_player = current turn player          # turn_hist.last(), default 0
    fired = []                                 # (effect_idx, card, controller)

    for each raised event (drain the queue):
        for each matching TRIGGER effect on the destroyed card:
            set activator = controller         # for the condition check
            if condition fails: skip
            if optional:  push OptionalTrigger  (yes/no)   # deferred — see Scope
            else:         fired.push (idx, card, controller)

    # SEGOC: turn player's triggers first. STABLE sort keeps event order
    # within a single player's triggers.
    fired.sort_by_key( (idx, card, controller) -> controller != turn_player )

    for (idx, card, controller) in fired:
        push_chain_link(idx, card, controller, no-targets)

    if fired is non-empty:
        push ResolveChain                       # unwinds LIFO once windows close
        push ChainResponse{ player = 1 - turn_player }   # opponent responds first
```

Why a **stable** sort: `false` (turn player) sorts before `true` (opponent), and
stability preserves the collection order (= event order) for triggers of the *same*
player — that's our stand-in for "the player chooses".

---

## Scope (this milestone)

- **Mandatory only** — buckets **1 & 2**. Optional triggers still resolve via the
  inline `OptionalTrigger` yes/no; SEGOC-ordering them (buckets 3 & 4) needs
  "optional triggers on the chain" first, which is deferred.
- **Targeting triggers deferred** — a trigger that targets would run its `target`
  stage before its link (like an activation). Our fixtures don't target.

## The observable (test)

Resolution *order* is hard to see with no-target effects (two board-wipes both wipe
regardless of order). So the SEGOC test inspects **placement order** directly:

```text
chain_activators() -> [controller of each link, in chain order]
```

Destroy the opponent's trigger monster *first*, then the turn player's, so event
order is the reverse of SEGOC order — then assert `chain_activators() == [TP, OPP]`.
That pins "turn player placed first" independent of event order.

---

## EDOPro reference

| Piece | EDOPro | file:line |
|---|---|---|
| detect events, bucket triggers | `process_instant_event` (`new_fchain` / `new_ochain`) | `processor.cpp:1173` |
| order + place on chain | `Processors::PointEvent` | `processor.cpp:587` |
| `current_player = turn player` | step 0 | `processor.cpp:604` |
| flip to the opponent | forced `:665`, optional `:765` | `processor.cpp:665` |
| all-forced-before-optional | forced loop drains both players first | `processor.cpp:664-668` |
| within-batch player choice | `SelectChain` | `processor.cpp:657` |
| TCG earliest-event narrowing | `DUEL_TCG_SEGOC_FIRSTTRIGGER` (sort by `event_id`) | `processor.cpp:646-651` |
| each accepted trigger → link | `AddChain` (same as manual activation) | `processor.cpp:679`, `:786` |
