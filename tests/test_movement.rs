//! Movement: cards move between zones. Destroy = send to the graveyard.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::PLAYER_0;

/// `send_to` moves a card to a new zone — it stays alive in the arena.
#[test]
fn send_to_moves_a_card_and_keeps_it_alive() {
    let mut duel = Duel::new();
    let c = duel.add_card(Card);
    duel.place(PLAYER_0, c, Zone::MonsterZone);

    duel.send_to(c, Zone::GY);

    assert_eq!(duel.zone_of(c), Some(Zone::GY), "moved to the graveyard");
    assert!(duel.get_card(c).is_some(), "moved, not deleted");
}

/// `send_to` works even on a card that had no prior zone.
#[test]
fn send_to_places_an_unzoned_card() {
    let mut duel = Duel::new();
    let c = duel.add_card(Card);

    duel.send_to(c, Zone::Hand);

    assert_eq!(duel.zone_of(c), Some(Zone::Hand));
}
