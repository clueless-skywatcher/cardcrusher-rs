//! The full loop: activating an effect freezes for a target, then resolves and
//! drives the real duel (ties the processor, freeze/resume, and the DSL together).

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;

/// Activate → the duel FREEZES to ask for a target (nothing destroyed yet) →
/// supply the target → resume → the effect resolves and destroys the card.
#[test]
fn activating_an_effect_freezes_for_a_target_then_resolves() {
    let mut duel = Duel::new();
    let monster = duel.add_card(Card);

    duel.load_card("cards/Example.rhai")
        .expect("Example.rhai should load");

    // Activate effect 0. It needs a target, so the duel freezes.
    assert_eq!(duel.activate(0), DuelStatus::Awaiting);
    assert!(
        duel.get_card(monster).is_some(),
        "nothing destroyed while we're still choosing a target"
    );
    assert_ne!(duel.zone_of(monster), Some(Zone::GY));

    // Choose the monster as the target.
    duel.answer_target(vec![monster]);

    // Resume: thaw → resolve runs → monster destroyed (sent to the graveyard).
    assert_eq!(duel.resume(), DuelStatus::End);
    assert_eq!(
        duel.zone_of(monster),
        Some(Zone::GY),
        "after resolving, the targeted monster should be in the graveyard"
    );
}
