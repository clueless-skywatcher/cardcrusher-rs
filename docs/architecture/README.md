# cardcrusher — Architecture (the definitive tour)

**What this is:** a top-to-bottom explanation of how the engine actually works —
every subsystem, with diagrams, real code, and step-by-step traces. Written to be
*read*, not skimmed-and-abandoned. (The terminal demo `examples/play.rs` is out of
scope here — this is the engine.)

---

## The one-paragraph mental model

cardcrusher is a **deterministic, headless rules engine**. It doesn't draw anything
and it doesn't ask *you* anything directly. Instead it runs a little **stack machine**:
it pulls the top task off a to-do stack, runs it one step, and either finishes,
asks a question (emits a **message**, waits for a **response**), or keeps going.
Cards are **Lua scripts** that describe what they do; the engine runs those scripts
and applies the results. That's the whole game in one breath.

```
        ┌─────────────────────────────────────────────┐
        │                   Duel                       │  ← owns EVERYTHING
        │  cards · field · life · the processor stack  │
        │  the Lua VM · the effect scratchpad          │
        └───────────────┬───────────────┬─────────────┘
        emits messages  │               │  runs Lua card scripts
                        ▼               ▼
              [ host / front-end ]   [ cards/*.lua ]
              answers with responses  describe effects
```

The engine speaks a **message/response protocol** — exactly what a real client (or a
network server) would use. `examples/play.rs` is just one such host.

---

## The sacred rule: **determinism**

Same inputs → *same* game, byte for byte. That's what makes replays and lockstep
networking possible. Everything below bends to it:

- one seeded PRNG · integers only · **sort before you iterate**
- **IDs, not pointers** (a card is a ticket you look up, not an address)
- `BTreeMap` / sorted `Vec`, **never `HashMap`** (hash order isn't stable)
- the Lua VM's garbage collector is **stopped** (GC pauses aren't deterministic)

If a change would make two identical games diverge, it's wrong. Chapter 1 explains
why each rule earns its place.

---

## The chapters (read in order)

| # | Chapter | The hook |
|---|---|---|
| 1 | [The Big Picture](01-the-big-picture.md) | what it is, the layers, the sacred rules, the module map |
| 2 | [The Duel & The Processor Loop](02-the-duel-and-the-loop.md) | the god object + the stack machine that *is* the game loop |
| 3 | [Cards, Zones & Movement](03-cards-zones-and-movement.md) | CardIds as tickets, the Field, and destroy-vs-send |
| 4 | [Scripting: Effects & Verbs](04-scripting-effects-and-verbs.md) | how a Lua card runs: stages, describe-then-execute, the coroutine pause |
| 5 | [The Chain Engine](05-the-chain-engine.md) | effects stack, the opponent responds, then it all resolves LIFO |
| 6 | [Events & Triggers](06-events-and-triggers.md) | "when X happens…", event detail bags, and the snapshot journey |
| 7 | [The Battle System](07-the-battle-system.md) | the attack step machine + the before/after-damage windows |
| 8 | [Modifiers & Continuous Effects](08-modifiers-and-continuous-effects.md) | standing conditions folded at query points (ATK, "no battle damage") |
| 9 | [Subscriptions & the Queue](09-subscriptions-and-queue.md) | "for the rest of this turn…" via queued closures |
| 10 | [Worked Example: Kuriboh](10-worked-example-kuriboh.md) | one card that touches *everything* — the grand tour |

**If you only read three:** 2 (the loop), 4 (how cards run), 10 (it all together).

---

## Two ideas that show up *everywhere*

You'll meet these in almost every chapter — learn them once here:

**1. Describe-then-execute.** A Lua verb (`e:destroy(...)`) never pokes the game
directly — it *records an intent* into a shared scratchpad (`EffectContext`). The
engine reads those intents back and applies them *after* the script stage finishes.
Why: it avoids a borrow cycle (Rust won't let the Lua callback and the `Duel` both
hold the game mutably), and it keeps "what a card wants" separate from "what the
engine does." (Chapter 4.)

**2. Push-in-reverse, run LIFO.** The processor stack runs top-first, so to make A
happen *before* B you push B **then** A. This one rule explains response windows,
chain resolution, and the battle step machine. (Chapters 2, 5, 7.)
