//! Card storage: the arena and stale-id safety.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;

/// Deleting a card makes its old id a tombstone: looking it up returns `None`
/// (never a crash, never a different card), and other cards are untouched.
#[test]
fn a_deleted_cards_id_resolves_to_nothing() {
    let mut duel = Duel::new();
    let a = duel.add_card(Card);
    let b = duel.add_card(Card);

    assert!(duel.get_card(a).is_some());
    assert!(duel.get_card(b).is_some());

    assert!(
        duel.remove_card(a).is_some(),
        "remove returns the removed card"
    );

    assert!(duel.get_card(a).is_none(), "stale id must resolve to None");
    assert!(duel.get_card(b).is_some(), "other card stays reachable");
}
