//! The game loop: the resumable to-do stack.

use cardcrusher::duel::{Duel, Winner};
use cardcrusher::processor::{
    DuelStatus, MSG_NEW_TURN, MSG_PHASE_BATTLE, MSG_PHASE_DRAW, MSG_PHASE_END, MSG_PHASE_MAIN1,
    MSG_PHASE_MAIN2, MSG_PHASE_STANDBY, MSG_SELECT_IDLECMD, MSG_STARTUP,
};
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Drive the duel to `End`, auto-answering every Main-Phase menu with
/// "go to next phase" (command 0). Panics if the loop ever yields `Continue`
/// (which `process` should never return — it loops internally).
fn run_answering_menus(duel: &mut Duel) {
    loop {
        match duel.process() {
            DuelStatus::End => break,
            DuelStatus::Awaiting => duel.set_response(&[0]),
            DuelStatus::Continue => unreachable!("process runs until End or Awaiting"),
        }
    }
}

/// A turn walks its phases, stopping at each Main Phase to offer the menu.
/// (The menu message follows the phase message it belongs to.)
#[test]
fn a_turn_walks_its_phases_and_stops_at_each_main_phase_menu() {
    let mut duel = Duel::new();
    duel.set_max_turns(1);
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(
        duel.messages(),
        [
            MSG_STARTUP,
            MSG_NEW_TURN,
            MSG_PHASE_DRAW,
            MSG_PHASE_STANDBY,
            MSG_PHASE_MAIN1,
            MSG_SELECT_IDLECMD, // Main Phase 1 menu
            MSG_PHASE_BATTLE,
            MSG_PHASE_MAIN2,
            MSG_SELECT_IDLECMD, // Main Phase 2 menu
            MSG_PHASE_END,
        ]
    );
}

/// The very first `process()` runs up to Main Phase 1 and freezes on the menu.
#[test]
fn a_turn_freezes_at_main_phase_1_for_the_menu() {
    let mut duel = Duel::new();
    duel.set_max_turns(1);
    duel.start();

    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(
        *duel.messages().last().unwrap(),
        MSG_SELECT_IDLECMD,
        "it should be waiting on the Main Phase menu"
    );
}

/// With nothing queued, the loop has nothing to do and reports `End`.
#[test]
fn an_empty_stack_reports_end() {
    let mut duel = Duel::new();
    assert_eq!(duel.process(), DuelStatus::End);
}

/// Once a winner is decided, the loop halts — no turns run and it reports `End`.
#[test]
fn no_turns_run_once_the_game_is_won() {
    let mut duel = Duel::new();
    duel.deal_damage(PLAYER_0, 8000); // player 1 has already won
    duel.start();

    assert_eq!(duel.process(), DuelStatus::End);
    assert!(
        duel.turn_history().is_empty(),
        "a decided game runs no turns"
    );
    assert_eq!(duel.result(), Some(Winner::Player(PLAYER_1)));
}

/// After a turn ends, play passes to the other player.
#[test]
fn play_passes_to_the_other_player_each_turn() {
    let mut duel = Duel::new();
    duel.set_max_turns(2);
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(duel.turn_history(), [PLAYER_0, PLAYER_1]);
}

/// `max_turns` bounds how many turns run; players keep alternating.
#[test]
fn max_turns_bounds_the_number_of_turns() {
    let mut duel = Duel::new();
    duel.set_max_turns(3);
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(duel.turn_history(), [PLAYER_0, PLAYER_1, PLAYER_0]);
}

/// A single-turn game belongs to player 0 only.
#[test]
fn a_single_turn_belongs_to_player_zero() {
    let mut duel = Duel::new();
    duel.set_max_turns(1);
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(duel.turn_history(), [PLAYER_0]);
}
