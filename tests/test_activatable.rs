//! `activatable_effects` should only offer effects that could actually DO
//! something — today it just checks `condition`, so a spell with no legal target
//! is still offered (and activating it silently fizzles). These pin that a
//! no-target effect is hidden. Mirrors EDOPro `effect::is_activateable`, which
//! also verifies the target count before offering an effect.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Example Spell targets "1 monster your opponent controls". With no such
/// monster on the field, its target set is empty → it must NOT be offered.
#[test]
fn no_legal_target_means_not_activatable() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/ExampleSpell.lua")
        .expect("ExampleSpell.lua should load");

    let spell = duel.make_card(12345678);
    duel.add_to_hand(PLAYER_0, spell);

    assert!(
        duel.activatable_effects(PLAYER_0).is_empty(),
        "a spell with no legal target should not be offered",
    );
}

/// Give the opponent a monster and the same spell becomes activatable — the fix
/// must not over-filter effects that DO have a target.
#[test]
fn a_legal_target_makes_it_activatable() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/ExampleSpell.lua")
        .expect("ExampleSpell.lua should load");

    let spell = duel.make_card(12345678);
    duel.add_to_hand(PLAYER_0, spell);

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);

    assert_eq!(
        duel.activatable_effects(PLAYER_0).len(),
        1,
        "with a legal target, the spell is offered",
    );
}
