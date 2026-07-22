//! First light for the Lua card engine (PHASE-LUA M1).
//!
//! Loading a card runs its Lua source. The card builds itself with
//! `Card:new(id)` and registers each effect with `add_effect()`. After loading,
//! the duel should know about those effects.
//!
//! This pins the M1 contract only: load + register. No resolving, no cost, no
//! targeting yet — those are M2+.

use cardcrusher::duel::Duel;

/// Loading `Example.lua` runs it, and its single `add_effect()` registers one
/// effect on the duel.
#[test]
fn loading_a_card_registers_its_effect() {
    let mut duel = Duel::new();

    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    assert_eq!(
        duel.effect_count(),
        1,
        "the card's one add_effect() should register one effect"
    );
}
