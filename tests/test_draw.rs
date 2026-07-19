//! Drawing: a card moves from a player's deck to their hand.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Drawing takes one card off a player's deck and puts it in their hand:
/// deck −1, hand +1.
#[test]
fn drawing_moves_a_card_from_deck_to_hand() {
    let mut duel = Duel::new();
    duel.add_to_deck(PLAYER_0, Card);
    duel.add_to_deck(PLAYER_0, Card);

    assert_eq!(duel.deck_count(PLAYER_0), 2);
    assert_eq!(duel.hand_count(PLAYER_0), 0);

    duel.draw(PLAYER_0, 1);

    assert_eq!(duel.deck_count(PLAYER_0), 1, "deck loses one card");
    assert_eq!(duel.hand_count(PLAYER_0), 1, "hand gains one card");
}

/// Each player's deck and hand are separate — drawing for player 0 doesn't touch
/// player 1.
#[test]
fn drawing_only_affects_the_drawing_player() {
    let mut duel = Duel::new();
    duel.add_to_deck(PLAYER_0, Card);
    duel.add_to_deck(PLAYER_1, Card);

    duel.draw(PLAYER_0, 1);

    assert_eq!(duel.hand_count(PLAYER_0), 1);
    assert_eq!(duel.hand_count(PLAYER_1), 0, "player 1 untouched");
    assert_eq!(duel.deck_count(PLAYER_1), 1);
}
