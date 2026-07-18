//! The game loop: the resumable to-do stack.

use cardcrusher::duel::Duel;
use cardcrusher::processor::{
    DuelStatus, MSG_NEW_TURN, MSG_PHASE_BATTLE, MSG_PHASE_DRAW, MSG_PHASE_END, MSG_PHASE_MAIN1,
    MSG_PHASE_MAIN2, MSG_PHASE_STANDBY, MSG_STARTUP,
};

/// Booting runs Startup, which hands off to a Turn. The turn then walks through
/// its phases in order and the stack drains to `End`.
#[test]
fn a_turn_walks_through_all_phases_in_order() {
    let mut duel = Duel::new();
    duel.set_max_turns(1);
    duel.start();

    assert_eq!(duel.process(), DuelStatus::End);
    assert_eq!(
        duel.messages(),
        [
            MSG_STARTUP,
            MSG_NEW_TURN,
            MSG_PHASE_DRAW,
            MSG_PHASE_STANDBY,
            MSG_PHASE_MAIN1,
            MSG_PHASE_BATTLE,
            MSG_PHASE_MAIN2,
            MSG_PHASE_END,
        ],
        "the turn should pass through every phase in order"
    );
}

/// With nothing queued, the loop has nothing to do and reports `End`.
#[test]
fn an_empty_stack_reports_end() {
    let mut duel = Duel::new();
    assert_eq!(duel.process(), DuelStatus::End);
}

/// After a turn's End phase, play passes to the other player. Bounded by
/// `max_turns` so the game — which has no real end condition yet — doesn't loop
/// forever.
#[test]
fn play_passes_to_the_other_player_each_turn() {
    let mut duel = Duel::new();
    duel.set_max_turns(2);
    duel.start();

    assert_eq!(duel.process(), DuelStatus::End);
    assert_eq!(
        duel.turn_history(),
        [0, 1],
        "player 0 takes turn 1, player 1 takes turn 2"
    );
}

/// `max_turns` bounds how many turns run; players keep alternating.
#[test]
fn max_turns_bounds_the_number_of_turns() {
    let mut duel = Duel::new();
    duel.set_max_turns(3);
    duel.start();

    assert_eq!(duel.process(), DuelStatus::End);
    assert_eq!(duel.turn_history(), [0, 1, 0], "three turns, alternating");
}

/// A single-turn game belongs to player 0 only.
#[test]
fn a_single_turn_belongs_to_player_zero() {
    let mut duel = Duel::new();
    duel.set_max_turns(1);
    duel.start();

    assert_eq!(duel.process(), DuelStatus::End);
    assert_eq!(duel.turn_history(), [0]);
}
