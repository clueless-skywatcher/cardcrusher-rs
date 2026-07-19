//! Summoning: a monster moves from a player's hand to the field.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::PLAYER_0;

/// Normal-summoning a card in hand puts it into a monster zone and takes it out
/// of the hand.
#[test]
fn normal_summon_moves_a_card_from_hand_to_the_field() {
    let mut duel = Duel::new();

    // Give player 0 a card in hand (draw it off the deck).
    duel.add_to_deck(PLAYER_0, Card);
    let monster = duel.draw(PLAYER_0, 1)[0];
    assert_eq!(duel.zone_of(monster), Some(Zone::Hand));
    assert_eq!(duel.hand_count(PLAYER_0), 1);

    duel.summon(monster);

    assert_eq!(
        duel.zone_of(monster),
        Some(Zone::MonsterZone),
        "now on the field"
    );
    assert_eq!(duel.hand_count(PLAYER_0), 0, "and gone from the hand");
}
