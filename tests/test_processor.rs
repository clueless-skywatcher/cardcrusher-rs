//! The game loop: the resumable to-do stack.

use cardcrusher::card::Card;
use cardcrusher::duel::{Duel, WinReason, Winner};
use cardcrusher::processor::DuelStatus;
use cardcrusher::{
    CMD_NEXT_PHASE, MSG_NEW_TURN, MSG_PHASE_BATTLE, MSG_PHASE_DRAW, MSG_PHASE_END, MSG_PHASE_MAIN1,
    MSG_PHASE_MAIN2, MSG_PHASE_STANDBY, MSG_SELECT_IDLECMD, MSG_STARTUP, PLAYER_0, PLAYER_1,
};

/// Drive the duel to `End`, auto-answering every Main-Phase menu with
/// "go to next phase" (command 0). Panics if the loop ever yields `Continue`
/// (which `process` should never return — it loops internally).
fn run_answering_menus(duel: &mut Duel) {
    loop {
        match duel.process() {
            DuelStatus::End => break,
            DuelStatus::Awaiting => duel.set_response(&[CMD_NEXT_PHASE]),
            DuelStatus::Continue => unreachable!("process runs until End or Awaiting"),
        }
    }
}

/// Give a player `n` cards to draw, so the Draw Phase doesn't deck them out.
fn stock_deck(duel: &mut Duel, player: usize, n: usize) {
    for _ in 0..n {
        duel.add_to_deck(player, Card);
    }
}

/// A turn walks its phases, stopping at each Main Phase to offer the menu.
/// (The menu message follows the phase message it belongs to.)
#[test]
fn a_turn_walks_its_phases_and_stops_at_each_main_phase_menu() {
    let mut duel = Duel::new();
    duel.set_max_turns(1);
    stock_deck(&mut duel, PLAYER_0, 5);
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
    stock_deck(&mut duel, PLAYER_0, 5);
    duel.start();

    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(
        *duel.messages().last().unwrap(),
        MSG_SELECT_IDLECMD,
        "it should be waiting on the Main Phase menu"
    );
}

/// The opening player skips their turn-1 draw; later turns draw normally.
#[test]
fn opening_player_skips_first_draw_then_later_turns_draw() {
    let mut duel = Duel::new();
    duel.set_max_turns(2);
    stock_deck(&mut duel, PLAYER_0, 5);
    stock_deck(&mut duel, PLAYER_1, 5);
    duel.start();

    run_answering_menus(&mut duel);

    // Player 0 went first → no turn-1 draw → deck untouched.
    assert_eq!(
        duel.deck_count(PLAYER_0),
        5,
        "opening player skips their first draw"
    );
    assert_eq!(duel.hand_count(PLAYER_0), 0);

    // Player 1's turn-2 Draw Phase drew a card.
    assert_eq!(duel.hand_count(PLAYER_1), 1, "later turns draw");
    assert_eq!(duel.deck_count(PLAYER_1), 4);
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
    stock_deck(&mut duel, PLAYER_0, 5);
    stock_deck(&mut duel, PLAYER_1, 5);
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(duel.turn_history(), [PLAYER_0, PLAYER_1]);
}

/// `max_turns` bounds how many turns run; players keep alternating.
#[test]
fn max_turns_bounds_the_number_of_turns() {
    let mut duel = Duel::new();
    duel.set_max_turns(3);
    stock_deck(&mut duel, PLAYER_0, 5);
    stock_deck(&mut duel, PLAYER_1, 5);
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(duel.turn_history(), [PLAYER_0, PLAYER_1, PLAYER_0]);
}

/// A single-turn game belongs to player 0 only.
#[test]
fn a_single_turn_belongs_to_player_zero() {
    let mut duel = Duel::new();
    duel.set_max_turns(1);
    stock_deck(&mut duel, PLAYER_0, 5);
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(duel.turn_history(), [PLAYER_0]);
}

/// A duel ends mid-play by deck-out: player 1 has no cards, so when their Draw
/// Phase comes they deck out and player 0 wins — no `max_turns` needed to stop it.
#[test]
fn a_duel_ends_by_deck_out_during_play() {
    let mut duel = Duel::new();
    duel.set_max_turns(5); // backstop; deck-out should end it first
    stock_deck(&mut duel, PLAYER_0, 5);
    // PLAYER_1's deck is empty on purpose.
    duel.start();

    run_answering_menus(&mut duel);

    assert_eq!(duel.result(), Some(Winner::Player(PLAYER_0)));
    assert_eq!(duel.win_reason(), Some(WinReason::DeckOut));
}
