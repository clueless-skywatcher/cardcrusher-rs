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

/// Removing the same card twice: the card the first time, `None` the second.
#[test]
fn removing_the_same_card_twice_yields_none_the_second_time() {
    let mut duel = Duel::new();
    let a = duel.add_card(Card);

    assert!(
        duel.remove_card(a).is_some(),
        "first removal returns the card"
    );
    assert!(
        duel.remove_card(a).is_none(),
        "second removal finds nothing"
    );
}

/// If a slot is reused after deletion, the new occupant gets a fresh generation,
/// so the old id stays dead — it never accidentally points at the new card.
#[test]
fn a_reused_slot_does_not_revive_an_old_id() {
    let mut duel = Duel::new();
    let a = duel.add_card(Card);
    duel.remove_card(a);

    let b = duel.add_card(Card); // may reuse a's freed slot

    assert!(
        duel.get_card(a).is_none(),
        "old id stays dead even if the slot is reused"
    );
    assert!(duel.get_card(b).is_some());
    assert_ne!(a, b, "the reused slot yields a distinct id");
}
