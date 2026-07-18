//! The card DSL: scripts describe effects, and stored `resolve` closures run
//! later and drive the real duel.

use std::{cell::RefCell, rc::Rc};

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::script::CardLibrary;

/// Loading a card runs its entry function, which describes the card by
/// registering exactly one activate effect. Registering does nothing else yet.
#[test]
fn loading_a_card_registers_its_effect() {
    let duel = Rc::new(RefCell::new(Duel::new()));
    let mut lib = CardLibrary::new(duel);

    lib.load_file("cards/Example.rhai")
        .expect("Example.rhai should load");

    assert_eq!(lib.effect_count(), 1);
}

/// The payoff: a `resolve` closure defined when the card loaded runs LATER and
/// destroys the targeted card in the shared, real duel.
#[test]
fn resolving_an_effect_destroys_the_targeted_card() {
    let duel = Rc::new(RefCell::new(Duel::new()));
    let monster = duel.borrow_mut().add_card(Card);
    assert!(duel.borrow().get_card(monster).is_some());

    let mut lib = CardLibrary::new(duel.clone());
    lib.load_file("cards/Example.rhai")
        .expect("Example.rhai should load");

    lib.answer_target(vec![monster]);
    lib.resolve(0);

    assert!(
        duel.borrow().get_card(monster).is_none(),
        "resolve should have destroyed the real card"
    );
}
