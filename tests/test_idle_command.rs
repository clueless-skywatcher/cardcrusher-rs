//! The Main-Phase menu: the engine asks the player what they want to do, then
//! acts on the answer.

use cardcrusher::duel::Duel;
use cardcrusher::processor::{DuelStatus, MSG_SELECT_IDLECMD};

/// The idle command lists the options and FREEZES for a choice; answering
/// "go to next phase" lets the turn move on.
#[test]
fn the_menu_asks_then_advances_when_told_to_move_on() {
    let mut duel = Duel::new();
    duel.idle_command();

    // Step 0: offer the menu, then freeze waiting for a choice.
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(duel.messages(), [MSG_SELECT_IDLECMD], "it asked the menu");

    // The player picks "go to next phase" (command 0).
    duel.set_response(&[0]);

    // Resume → the menu is satisfied, stack drains.
    assert_eq!(duel.process(), DuelStatus::End);
}

/// A command other than "next phase" keeps the player in the Main Phase: the
/// menu is offered again, and only "next phase" (0) ends it.
#[test]
fn a_non_advancing_command_reopens_the_menu() {
    let mut duel = Duel::new();
    duel.idle_command();

    assert_eq!(duel.process(), DuelStatus::Awaiting);

    // Some action that isn't "next phase" → still in the Main Phase.
    duel.set_response(&[9]);
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(
        duel.messages(),
        [MSG_SELECT_IDLECMD, MSG_SELECT_IDLECMD],
        "the menu is shown again after a non-advancing command"
    );

    // Finally, go to the next phase.
    duel.set_response(&[0]);
    assert_eq!(duel.process(), DuelStatus::End);
}
