//! The payoff: activating a card's effect from the Main-Phase menu — the menu,
//! the processor, freeze/resume, and the DSL all working together.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::CMD_ACTIVATE;

/// Pick "activate effect 0" from the menu → the effect asks for a target (the
/// duel freezes) → answer it → resolve → the target is destroyed.
///
/// Response encoding: `[CMD_ACTIVATE, effect_index]`.
#[test]
fn activating_an_effect_from_the_menu_resolves_it() {
    let mut duel = Duel::new();
    let monster = duel.add_card(Card); // the effect's target
    duel.load_card("cards/Example.rhai")
        .expect("Example.rhai should load");

    // Open the Main-Phase menu; it freezes for a choice.
    duel.idle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);

    // Choose "activate effect 0". The effect needs a target, so it freezes again.
    duel.set_response(&[CMD_ACTIVATE, 0]);
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "activating should freeze to ask for the effect's target"
    );

    // Supply the target, then let it resolve.
    duel.answer_target(vec![monster]);
    assert_eq!(duel.process(), DuelStatus::End);

    assert_eq!(
        duel.zone_of(monster),
        Some(Zone::GY),
        "the activated effect destroyed its target"
    );
}
