//! The full loop: activating an effect freezes for a target, then resolves and
//! drives the real duel (ties the processor, freeze/resume, and the DSL together).

use std::{cell::RefCell, rc::Rc};

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::script::CardLibrary;

/// Activate → the duel FREEZES to ask for a target (nothing destroyed yet) →
/// supply the target → resume → the effect resolves and destroys the card.
#[test]
fn activating_an_effect_freezes_for_a_target_then_resolves() {
    let duel = Rc::new(RefCell::new(Duel::new()));
    let monster = duel.borrow_mut().add_card(Card);

    let mut lib = CardLibrary::new(duel.clone());
    lib.load_file("cards/Example.rhai")
        .expect("Example.rhai should load");

    // Activate effect 0. It needs a target, so the duel freezes.
    assert_eq!(lib.activate(0), DuelStatus::Awaiting);
    assert!(
        duel.borrow().get_card(monster).is_some(),
        "nothing should be destroyed while we're still choosing a target"
    );

    // Choose the monster as the target.
    lib.answer_target(vec![monster]);

    // Resume: thaw → the stored resolve closure runs → the monster is destroyed.
    assert_eq!(lib.resume(), DuelStatus::End);
    assert!(
        duel.borrow().get_card(monster).is_none(),
        "after resolving, the targeted monster should be gone"
    );
}
