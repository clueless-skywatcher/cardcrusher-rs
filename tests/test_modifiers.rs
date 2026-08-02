//! Per-card **modifiers** that stack, folded by a query gate. A modifier is tagged
//! with the **source** card that produced it, so all of one card's modifiers can be
//! removed together. The fold is ordered by each type's **priority** (ties → insertion
//! order): `SetAtk` is an *earlier* band than `AtkChange`, so a "becomes X" sets the
//! value and every "+N" stacks on top — EDOPro `get_attack`: final = set value + Σ
//! updates, floored at 0. Insertion order is irrelevant between SET and ADD.
//!
//! ASSUMED API (this test defines the contract — implement to match):
//! - `cardcrusher::modifiers::ModifierType` — enum: `AtkChange(i32)`, `DefChange(i32)`,
//!   `SetAtk(i32)`; plus `ModifierType::priority(&self) -> Option<i32>` — `SetAtk` an
//!   earlier band than `AtkChange`; `None` = unprioritized.
//! - `duel.add_modifier(card: CardId, source: CardId, kind: ModifierType)` — append to
//!   `card`'s list, tagged with the `source` card that produced it.
//! - `duel.atk_of(card)` / `def_of(card)` — fold sorted by (priority, insertion order),
//!   floored at 0.
//! - `duel.remove_modifiers_from(source)` — drop every modifier, on *any* card, whose
//!   source is `source`.

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::ids::CardId;
use cardcrusher::modifiers::ModifierType;

/// A card with the given printed ATK.
fn card_with_atk(duel: &mut Duel, atk: i32) -> CardId {
    duel.add_card(Card::with_data(
        1,
        CardData {
            atk,
            ..Default::default()
        },
    ))
}

/// A bare card to stand in as the *source* of a modifier (whatever produced it).
fn source(duel: &mut Duel) -> CardId {
    duel.add_card(Card::new(0))
}

// ===== Folding (stacking) ===================================================

/// One modifier shifts the queried ATK.
#[test]
fn atk_reflects_a_modifier() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let c = card_with_atk(&mut duel, 1000);

    duel.add_modifier(c, src, ModifierType::AtkChange(-100));

    assert_eq!(duel.atk_of(c), Some(900), "1000 − 100");
}

/// Modifiers **stack** — the gate sums them all.
#[test]
fn atk_modifiers_stack_additively() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let c = card_with_atk(&mut duel, 1000);

    duel.add_modifier(c, src, ModifierType::AtkChange(300));
    duel.add_modifier(c, src, ModifierType::AtkChange(-100));

    assert_eq!(duel.atk_of(c), Some(1200), "1000 + 300 − 100 = 1200");
}

/// Equal-and-opposite modifiers cancel to a net zero change (both still present).
#[test]
fn opposite_atk_modifiers_cancel() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let c = card_with_atk(&mut duel, 1000);

    duel.add_modifier(c, src, ModifierType::AtkChange(100));
    duel.add_modifier(c, src, ModifierType::AtkChange(-100));

    assert_eq!(duel.atk_of(c), Some(1000), "+100 and −100 net to 0");
}

/// ATK can't go negative — a big reduction floors at 0 (YGO ruling; EDOPro
/// `get_attack` clamps).
#[test]
fn atk_is_floored_at_zero() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let c = card_with_atk(&mut duel, 1000);

    duel.add_modifier(c, src, ModifierType::AtkChange(-1500));

    assert_eq!(duel.atk_of(c), Some(0), "1000 − 1500 floors at 0, not −500");
}

/// Modifiers are **per card** — one card's list doesn't touch another's.
#[test]
fn modifiers_are_per_card() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let a = card_with_atk(&mut duel, 1000);
    let b = card_with_atk(&mut duel, 1000);

    duel.add_modifier(a, src, ModifierType::AtkChange(500));

    assert_eq!(duel.atk_of(a), Some(1500), "A gets the boost");
    assert_eq!(duel.atk_of(b), Some(1000), "B is untouched");
}

// ===== Priority: SetAtk is the base, AtkChange stacks on top =================
// Priority orders the fold (ties → insertion). SetAtk is an earlier band than
// AtkChange, so a "becomes X" sets the value and every "+N" stacks on top —
// EDOPro get_attack: final = set value + Σ updates. Insertion order is irrelevant
// between the two.

/// A set establishes the value; an add stacks on top of it.
#[test]
fn a_set_is_the_base_and_adds_stack_on_top() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let c = card_with_atk(&mut duel, 1000);

    duel.add_modifier(c, src, ModifierType::SetAtk(0));
    duel.add_modifier(c, src, ModifierType::AtkChange(500));

    assert_eq!(
        duel.atk_of(c),
        Some(500),
        "becomes 0, then +500 stacks → 500"
    );
}

/// Priority beats insertion order: even when the add is registered FIRST, the set
/// still applies before it (naive insertion order would give 0).
#[test]
fn a_set_applies_before_an_add_regardless_of_insertion() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let c = card_with_atk(&mut duel, 1000);

    duel.add_modifier(c, src, ModifierType::AtkChange(500)); // registered first...
    duel.add_modifier(c, src, ModifierType::SetAtk(0)); // ...but SET's band is earlier

    assert_eq!(
        duel.atk_of(c),
        Some(500),
        "SET is the base (applies first), +500 stacks → 500, not 0",
    );
}

// ===== Removal by source ====================================================

/// `remove_modifiers_from` drops only the given source's modifiers; others stay.
#[test]
fn remove_modifiers_from_drops_only_that_sources_mods() {
    let mut duel = Duel::new();
    let src_a = source(&mut duel);
    let src_b = source(&mut duel);
    let c = card_with_atk(&mut duel, 1000);

    duel.add_modifier(c, src_a, ModifierType::AtkChange(500)); // +500 from A
    duel.add_modifier(c, src_b, ModifierType::AtkChange(300)); // +300 from B
    assert_eq!(duel.atk_of(c), Some(1800), "both apply");

    duel.remove_modifiers_from(src_a);

    assert_eq!(duel.atk_of(c), Some(1300), "A's +500 gone, B's +300 stays");
}

/// A source can buff many cards; removing it clears them **all**.
#[test]
fn remove_modifiers_from_spans_every_affected_card() {
    let mut duel = Duel::new();
    let src = source(&mut duel);
    let x = card_with_atk(&mut duel, 1000);
    let y = card_with_atk(&mut duel, 2000);

    duel.add_modifier(x, src, ModifierType::AtkChange(500));
    duel.add_modifier(y, src, ModifierType::AtkChange(500));

    duel.remove_modifiers_from(src);

    assert_eq!(duel.atk_of(x), Some(1000), "x cleared");
    assert_eq!(duel.atk_of(y), Some(2000), "y cleared");
}
