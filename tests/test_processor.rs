//! The game loop: the resumable to-do stack.

use cardcrusher::duel::Duel;
use cardcrusher::processor::{DuelStatus, MSG_NEW_TURN, MSG_STARTUP};

/// Booting queues one Startup task; it runs, hands off to a Turn, the stack
/// drains, and the loop reports `End` — with messages emitted in order.
#[test]
fn booting_runs_startup_then_turn_then_ends() {
    let mut duel = Duel::new();
    duel.start();

    assert_eq!(duel.process(), DuelStatus::End);
    assert_eq!(
        duel.messages(),
        [MSG_STARTUP, MSG_NEW_TURN],
        "Startup should run before Turn"
    );
}

/// With nothing queued, the loop has nothing to do and reports `End`.
#[test]
fn an_empty_stack_reports_end() {
    let mut duel = Duel::new();
    assert_eq!(duel.process(), DuelStatus::End);
}
