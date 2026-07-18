//! Drawing: a card moves from a player's deck to their hand.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;

/// Drawing takes one card off a player's deck and puts it in their hand:
/// deck −1, hand +1.
#[test]
fn drawing_moves_a_card_from_deck_to_hand() {
    let mut duel = Duel::new();
    duel.add_to_deck(0, Card);
    duel.add_to_deck(0, Card);

    assert_eq!(duel.deck_count(0), 2);
    assert_eq!(duel.hand_count(0), 0);

    duel.draw(0);

    assert_eq!(duel.deck_count(0), 1, "deck loses one card");
    assert_eq!(duel.hand_count(0), 1, "hand gains one card");
}

/// Each player's deck and hand are separate — drawing for player 0 doesn't touch
/// player 1.
#[test]
fn drawing_only_affects_the_drawing_player() {
    let mut duel = Duel::new();
    duel.add_to_deck(0, Card);
    duel.add_to_deck(1, Card);

    duel.draw(0);

    assert_eq!(duel.hand_count(0), 1);
    assert_eq!(duel.hand_count(1), 0, "player 1 untouched");
    assert_eq!(duel.deck_count(1), 1);
}
