//! The field: cards live in real zones.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::PLAYER_0;

/// A card reports the zone it was placed in; an unplaced card reports none; and
/// placing it again moves it.
#[test]
fn a_card_reports_the_zone_it_lives_in() {
    let mut duel = Duel::new();
    let c = duel.add_card(Card);

    // Freshly added, it hasn't been placed anywhere.
    assert_eq!(duel.zone_of(c), None);

    // Place it in a hand → it reports that zone.
    duel.place(PLAYER_0, c, Zone::Hand);
    assert_eq!(duel.zone_of(c), Some(Zone::Hand));

    // Placing again moves it.
    duel.place(PLAYER_0, c, Zone::MonsterZone);
    assert_eq!(duel.zone_of(c), Some(Zone::MonsterZone));
}

/// An unplaced card reports no zone.
#[test]
fn an_unplaced_card_has_no_zone() {
    let mut duel = Duel::new();
    let c = duel.add_card(Card);
    assert_eq!(duel.zone_of(c), None);
}

/// Cards track their zones independently of one another.
#[test]
fn cards_have_independent_zones() {
    let mut duel = Duel::new();
    let a = duel.add_card(Card);
    let b = duel.add_card(Card);

    duel.place(PLAYER_0, a, Zone::Hand);
    duel.place(PLAYER_0, b, Zone::GY);

    assert_eq!(duel.zone_of(a), Some(Zone::Hand));
    assert_eq!(duel.zone_of(b), Some(Zone::GY));
}
