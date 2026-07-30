# The chain engine

**Living doc.** Concepts behind cardcrusher's chain system — engine design first,
Yu-Gi-Oh! rules cited inline where they explain a choice. Grows one milestone at a
time. No implementation code lives here (that's the codebase); this is the *why*
and the *shape*.

## What the chain is for

Effects don't resolve the instant you activate them. They stack up as a **chain**,
the opponent gets a window to respond, and then the whole stack resolves
**last-in-first-out**. The engine's job is to model that stack + the response
window + the LIFO unwind.

```
instant (old):  activate A ─────────────────► resolve A
chain (new):    activate A → [respond?] → ... → resolve LIFO
```

## Status

| Milestone | Concept | State |
|---|---|---|
| **C0** | activation builds a chain; a separate step resolves it (LIFO) | ✅ done |
| **C1** | opponent response window (pass → resolve) | next |
| **C2** | chain a 2nd effect, gated by **spell speed** → resolve LIFO (merged w/ old C3) | in design |
| **C4** | triggers build a chain instead of resolving inline | deferred |
| **C5** | SEGOC — ordering many simultaneous triggers | deferred |

---

## C0 — the chain, and the activate/resolve split *(built)*

### Data model
- **`Duel.chain`** — the current chain, a growable list. Index 0 = bottom = the
  first link; the back = the most recent link.
- **A chain link** snapshots one activated effect awaiting resolution:
  - *which effect* — its index into the effect registry (so resolution knows what
    to run — **not** the link's position on the chain; those are different numbers).
  - *the card* — for its post-resolution lifecycle (Spell/Trap → GY).
  - *the activator* — the controlling player (drives YOU/OPPONENT relativity).
  - *a targets snapshot* — the cards chosen at activation.

> **Why snapshot targets/activator per link?** The effect scratchpad (`effect_ctx`)
> is a single shared object — each link overwrites it. So a link must carry its own
> copy and restore it right before that link resolves.

> EDOPro parallel: `struct chain` (a frozen snapshot of effect + targets + link #),
> living on `core.current_chain` — `field.h:48`, `field.h:239`.

### The key idea: split activation from resolution
Before C0, "activate" *was* "resolve". C0 breaks them apart:

```
activate  →  put a link on the chain   (do NOT resolve)
             ─────────────────────────
resolve    →  walk the chain, apply each link, empty it
```

That gap between "on the chain" and "resolved" is exactly where C1/C2 will insert
the response window. C0 leaves the gap empty.

### Resolution is LIFO
Resolving walks the chain **back-to-front** — pop the most recent link, resolve it,
then the next, until empty. Last activated resolves first.

> EDOPro parallel: `SolveChain` reverse-iterates (`rbegin()`) and pops the back —
> `processor.cpp:4096`.

Per link, resolution: restore that link's activator + targets into the shared
scratchpad → run the effect → a resolved Spell/Trap goes to the GY.

**Dry run (one link):**
```
start        Hand [Nuke]   OppField [Beaver]   chain []
activate     → link pushed                     chain [Nuke]     (Beaver still there!)
resolve      → pop Nuke, run it → Beaver → GY  chain []         Nuke → GY
```
The middle line is the whole point: the effect is *committed* but *not yet
resolved*. C1 puts the opponent's response window on that middle line.

### How it rides the processor stack
Reminder of the stack rule: **the top runs first**, and a task's children run
before the task's own next step. So to make things happen in order, you push them
in **reverse**.

C0 flow, when an effect is activated from the menu:

```
push IdleCommand   ← runs LAST  (back to the menu)
push ResolveChain  ← runs FIRST (drains the chain)
```

`ResolveChain` is a one-shot processor: it drains the whole chain, then finishes.
(C1 will slot a response-window processor *above* `ResolveChain`, so it runs first.)

---

## C1 / C2 — the response window (priority) *(confirmed vs EDOPro)*

C1 (one link, pass → resolve) and C2 (chain a 2nd link, LIFO) share **one priority
model**. Rules verified against `QuickEffect` (`processor.cpp:943`):

- **Opponent-of-the-adder goes first.** After X activates, the window opens for the
  *other* player — never for X (`processor.cpp:1147`). You don't respond to your
  own just-activated effect first.
- **Priority ping-pongs.** Every add *and* every pass re-opens the window for the
  other player (`:1147` add, `:1153` pass): X → Y → X → …
- **Two passes IN A ROW close it** → resolve. EDOPro sets a pass-flag per player;
  the window closes only when **both** flags are set (`:1151`–`:1154`).
- **Adding a link RESETS both flags** (`:1148`–`:1149`). So "in a row" = two
  consecutive passes with *no* activation between. A player who passed earlier gets
  a fresh window if the opponent then chains.

**Dry run — two links, P1's turn:**
```
P1 activates A     chain [A]        window → P2    (flags reset)
P2 chains B        chain [A,B]      window → P1    (flags reset)
P1 chains C        chain [A,B,C]    window → P2    (flags reset)
P2 passes          flag[P2]=1       window → P1    (still open)
P1 passes          flag[P1]=1       BOTH set → CLOSE
resolve LIFO       C → B → A
```

**Decisions so far:**
- **Pass state** = a **two-flag pair** (one per player), reset to `[F,F]` on every
  add; window closes when both are `T`. Lives on the **`Duel`** (must outlive the
  many response windows a ping-pong spawns).
- **Prompt** = the responder's currently-**chainable** effects + a **Pass** option.
  ("Chainable" is defined by spell speed — see below.)
- **C2 red test** = the ping-pong trace (A → B → C → pass → pass → resolve C,B,A).

## Spell speed — the response gate *(merged into C2)*

Every effect has a **spell speed 0–3**; it decides what may chain onto what.

> EDOPro *derives* speed (`effect::get_speed()`, `effect.cpp:694`), doesn't store it:
> **0** monster activate / static (not chainable) · **1** ignition, triggers,
> normal spells · **2** quick effects, quick-play spells, normal traps ·
> **3** counter traps.

**The gate** (`effect::is_chainable`, `effect.cpp:511`):
- **Speed 1 can only START a chain** — never a response.
- To chain onto an existing link, your speed must be **≥ the top link's speed**.

```
top = normal spell (1)  → respond w/ quick(2) ✓   another SS1 ✗
top = quick (2)         → quick(2) ✓  counter(3) ✓   SS1 ✗
top = counter trap (3)  → counter(3) ✓              quick(2) ✗
```

So the response prompt = "effects the responder can activate whose spell speed
passes this gate against the current top link."

**Decided: DERIVE** from `effect.kind` + the owning card's class + **subtype enum**.

**Design choice (diverges from EDOPro):** instead of packing subtypes into the
`TYPE_` bitmask (EDOPro's `TYPE_QUICKPLAY` / `TYPE_COUNTER` …), we use a **per-class
subtype enum** — `spell_type = SPELL_*` (`spell_types.lua`: NORMAL/QUICKPLAY/
CONTINUOUS/FIELD/EQUIP/RITUAL/QUEST), and a parallel `trap_type = TRAP_*` to add
(NORMAL/CONTINUOUS/COUNTER). These enums are the **single source of truth** for
subtype; the `TYPE_QUICKPLAY`/`TYPE_COUNTER` bits are *not* consulted for speed.

**The derivation (truth table):**

| kind | class · subtype | speed |
|---|---|---|
| `QUICK` | any | 2 |
| `IGNITION` | any | 1 |
| `TRIGGER` | any | 1 |
| `ACTIVATE` | spell · `SPELL_QUICKPLAY` | 2 |
| `ACTIVATE` | spell · else (normal/continuous/field/equip/ritual/quest) | 1 |
| `ACTIVATE` | trap · `TRAP_COUNTER` | 3 |
| `ACTIVATE` | trap · else (normal/continuous) | 2 |
| `ACTIVATE` | monster | 0 (shouldn't occur — monster effects are IGNITION/TRIGGER) |

Speed is a **pure function of (kind, class, subtype)** → trivially unit-testable.

**Plumbing gaps (must close before deriving):**
- `spell_type` / `trap_type` are Lua-only — `register_card` harvests just `type`
  into `CardData.card_type`. Add `spell_type` (+ `trap_type`) to `CardData` and
  harvest them.
- Ensure `spell_types.lua` (and a future `trap_types.lua`) is `include_str!`'d in
  `load_prelude`, or the `SPELL_*` names are undefined at card load.

### New verb: `send` — move a card anywhere

The YGO-real save needs the responder to move a card **off the field**. Generalize
it: a `send(card, zone)` verb that relocates a card to **any** zone — hand, deck,
GY, or banishment.

- **It's a thin wrapper over the engine's existing `send_to(card, zone)`** (already
  used internally for the S/T→GY lifecycle). Follows describe-then-execute like
  `destroy`: the verb records `(card, zone)` into the scratchpad; the `Duel` applies
  it after the stage (a `to_move` list drained like `to_destroy`).
- **`send` is NOT `destroy`.** Sending a card to the GY this way is a plain *send*
  (discard/tribute/return) — no `REASON_DESTROY`, no `EVENT_DESTROYED`. The YGO rule:
  "sent to the GY" ≠ "destroyed", and the two must stay distinct (triggers care).
- **Name:** `send(card, zone)`.
- **Zone constants:** add a `zones.lua` with `ZONE_*` (hand/deck/GY/banish/…)
  mapping to the Rust `Zone` enum (none exist in the prelude yet).

### Fixtures still to design
- The **speed-2 responder**: a quick-play spell (`ACTIVATE` + `spell_type =
  SPELL_QUICKPLAY` → speed 2) that `send`s a monster you control to hand. Note it's
  kind `ACTIVATE` (a spell *activation*), **not** kind `QUICK` — so it exercises the
  `ACTIVATE · SPELL_QUICKPLAY` row of the truth table, the more interesting one.
- Optionally a **speed-3 counter trap** later, to show it out-speeds a quick.

### C2 build order (dependency-first)

1. **Plumbing** ✅ — `spell_type`/`trap_type` on `CardData` + harvested; `level` now
   `Option`; `spell_types.lua` + `trap_types.lua` included in `load_prelude`.
2. **Spell speed** — a pure `(kind, class, subtype) → 0..3` derive; unit-test it in
   isolation against the truth table.
3. **`send` verb** — `zones.lua`, a `to_move` scratchpad list drained like
   `to_destroy`, wired to `send_to`.
4. **Chainability gate** — "speed ≥ top link, and SS1 never responds"; this is what
   `ChainResponse` uses to build the responder's chainable list.
5. **`ChainResponse` processor** — C1 first (offer opponent → pass → resolve), then
   C2 (ping-pong, two-flag pair on the `Duel`).
6. **Fixtures + red tests** — Emergency-Bounce (quick-play, `send` to hand); the
   ping-pong LIFO test.

Each rung is independently testable, so red-first works at every step.

---

## EDOPro reference map

Our simplified pieces vs. the real `ygopro-core` (for when we need to check a ruling
or mechanism):

| Our piece | EDOPro | file:line |
|---|---|---|
| chain link (snapshot) | `struct chain` | `field.h:48` |
| the current chain | `chain_array current_chain` | `field.h:239` |
| activation → link | `Processors::AddChain` (push → cost → target) | `processor.cpp:3626` |
| response window | `Processors::QuickEffect` (ping-pong; both pass → resolve) | `processor.cpp:943` |
| resolve LIFO | `Processors::SolveChain` (`rbegin`, pop back) | `processor.cpp:4096` |
| spell speed (C3) | `effect::get_speed()` → 0/1/2/3 | `effect.cpp:694` |
| chainability (C3) | `effect::is_chainable()` (new speed ≥ top link) | `effect.cpp:511` |
| triggers → chain (C4/C5) | `process_instant_event` → `Processors::PointEvent` | `processor.cpp:1173`, `587` |

**Spell speeds** (EDOPro derives, doesn't store): 0 = non-chainable (monster
activate); 1 = ignition, triggers, normal spells; 2 = quick effects, quick-play
spells, normal traps; 3 = Counter Traps.
