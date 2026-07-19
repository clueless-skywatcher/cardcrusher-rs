//! Game-end conditions: a player loses at 0 life points or on a deck-out; both
//! losing at once is a draw. Mirrors EDOPro's win-check (winner + reason, with a
//! tie when both conditions hit together).

use cardcrusher::duel::{Duel, WinReason, Winner};
use cardcrusher::{PLAYER_0, PLAYER_1};

/// No one has won at the start.
#[test]
fn no_winner_at_the_start() {
    let duel = Duel::new();
    assert_eq!(duel.result(), None);
}

/// A player reduced to 0 life points loses — the other player wins, reason LP.
#[test]
fn zero_life_points_loses_the_game() {
    let mut duel = Duel::new();

    duel.deal_damage(PLAYER_0, 8000);

    assert_eq!(duel.life_points(PLAYER_0), 0);
    assert_eq!(duel.result(), Some(Winner::Player(PLAYER_1)));
    assert_eq!(duel.win_reason(), Some(WinReason::LifePointsDepleted));
}

/// A player who must draw from an empty deck decks out and loses, reason deck-out.
#[test]
fn decking_out_loses_the_game() {
    let mut duel = Duel::new();

    // Player 0's deck is empty; trying to draw decks them out.
    duel.draw(PLAYER_0, 1);

    assert_eq!(duel.result(), Some(Winner::Player(PLAYER_1)));
    assert_eq!(duel.win_reason(), Some(WinReason::DeckOut));
}

/// Both players hitting 0 at once is a draw.
#[test]
fn both_players_at_zero_is_a_draw() {
    let mut duel = Duel::new();

    duel.deal_damage(PLAYER_0, 8000);
    duel.deal_damage(PLAYER_1, 8000);

    assert_eq!(duel.result(), Some(Winner::Draw));
}
