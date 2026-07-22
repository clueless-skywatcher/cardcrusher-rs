# Phase 2 — Turns & Operations

Phase 1 proved the skeleton: arena, state machine, freeze/resume, and the card
DSL, all talking. Phase 2 makes it **play**: a real turn clock, real zones, and
the moves that push cards around the board.

By the end you can run a scripted mini-duel — summon a monster, activate a spell,
draw, take damage — through the engine loop.

> **How to read this:** each milestone's **Done when** is a behaviour test. Write
> that test first (red), then implement until it passes (green). One phase per
> test, small steps.

---

## The map

```
Turn clock ──► Zones ──► Movement ──► Draw / Damage ──► Main-phase menu ──► Cards
  (M1–M2)      (M3)       (M4)          (M5–M6)            (M7)             (M8)
```

- **M1–M2** — time: phases advance, players alternate.
- **M3** — space: cards live in real zones.
- **M4** — motion: cards move between zones (destroy = send to graveyard).
- **M5–M6** — resources: cards in hand, life points going down.
- **M7** — choice: the player's action menu each Main Phase.
- **M8** — payoff: a handful of real cards playable end-to-end.

---

## Milestone 1 — The phase clock

**In plain words:** a turn walks through its phases instead of ending instantly.

**Phases:** Draw → Standby → Main 1 → Battle → Main 2 → End.

**What you'll build:** phase message constants, and a `Turn` task that emits one
phase per `step` (pausing between, finishing on End). See `TURN-FLOW.md`.

**Done when:** booting produces the full phase sequence in order.

**Watch out for:** `Turn` never needs an answer — pausing just loops, it doesn't
freeze.

---

## Milestone 2 — Player handover

**In plain words:** after the End Phase, it becomes the *other* player's turn.

**What you'll build:** a `turn_player` (0 or 1) the `Turn` task flips at handover,
then restarts itself for the new player — one `Turn` object ping-ponging.

**Done when:** two turns run back-to-back; turn 1 belongs to player 0, turn 2 to
player 1.

**Watch out for:** naïve handover **loops forever** (nothing ends the game yet).
Bound it for the test — e.g. stop after N turns, or a `max_turns` guard.

---

## Milestone 3 — Real zones on the field

**In plain words:** cards stop floating in a bare arena and start living somewhere
— a hand, a deck, a monster zone, a graveyard — one set per player.

**What you'll build:** a `Zone` enum (`Deck`, `Hand`, `MonsterZone`, `Graveyard`,
…), and a card's *location* (`owner` + `zone`). The `Field` tracks which cards are
where.

**Done when:** a card placed in a zone reports that zone; a card with no zone
reports none.

**Watch out for:** determinism — ordered collections for zone contents, never a
hash set.

---

## Milestone 4 — Movement primitives (SendTo)

**In plain words:** one function moves a card from wherever it is to a new zone.
Everything else (summon, destroy, discard) is built on it.

**What you'll build:** `send_to(card, zone)` — updates the card's location and the
field. Redefine **destroy** as "send to the graveyard" (not just remove).

**Done when:** destroying a monster moves it to its owner's graveyard — it's gone
from the monster zone but *findable* in the graveyard.

**Watch out for:** this changes Phase 1's `Destroy`, which just removed the card.
Update the DSL `Destroy` to route through `send_to(..., Graveyard)`.

---

## Milestone 5 — Draw

**In plain words:** the Draw Phase moves the top card of the deck into the hand.

**What you'll build:** a `draw(player, n)` op; wire it into the Draw Phase step.

**Done when:** after the Draw Phase, one card moved from deck to hand (deck −1,
hand +1).

**Watch out for:** drawing from an empty deck is a **loss** (deck-out) — leave a
`TODO` for the loss check, or wire a minimal one.

---

## Milestone 6 — Life points & damage

**In plain words:** each player has life points; costs and damage lower them; 0 is
a loss.

**What you'll build:** `life_points[2]`, a `pay_lp` / `deal_damage` op. Make the
DSL's `PayLP(n)` actually deduct.

**Done when:** `PayLP(500)` drops the player's LP by exactly 500.

**Watch out for:** integers only (determinism). No fractional life.

---

## Milestone 7 — The Main-Phase menu

**In plain words:** in a Main Phase the engine asks *"what do you want to do?"* —
summon, activate, set, or move to the next phase — and acts on the answer.

**What you'll build:** an `IdleCommand` task: step 0 lists the legal options and
freezes; step 1 reads the choice and queues the matching task (summon, activate…).

**Done when:** in Main 1 the engine freezes asking for an idle command; answering
"go to next phase" advances; answering "activate" runs an effect.

**Watch out for:** the response packs *two* numbers (what + which). Validate before
acting — reject illegal choices.

---

## Milestone 8 — A few real cards

**In plain words:** prove the whole thing with actual cards played through the loop.

**What you'll build:** ~5–10 cards in the DSL — a vanilla monster (Normal Summon),
a removal spell, a draw spell — and a stub host loop that plays them.

**Done when:** an integration test plays a short scripted game: summon a monster,
activate a spell that destroys it, draw a card — all through `process()`.

**Watch out for:** keep the cardpool tiny and scoped. Breadth is Phase 5.

---

## Order & dependencies

```
M1 phase clock
   │
M2 handover
   │
M3 zones ───► M4 movement ───► M5 draw
                   │              │
                   └──► M6 life points
                          │
                   M7 main-phase menu
                          │
                   M8 real cards (needs M1–M7)
```

Do M1 → M2 → M3 → M4 in order. M5/M6 can go in parallel after M4. M7 needs the
freeze/resume from Phase 1 plus M3–M4. M8 ties everything together.

---

## Out of scope for Phase 2

- ❌ The chain / priority / SEGOC response system (Phase 3 — the hard one).
- ❌ The full damage step and battle sub-timings (Phase 4).
- ❌ Broad card coverage (Phase 5).

Phase 2 is a **playable slice**, not the whole rulebook.
