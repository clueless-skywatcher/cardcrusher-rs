//! Acting from the Main-Phase menu: summoning a monster.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_SUMMON, PLAYER_0};

/// Answering the menu with "summon the card at hand slot 0" puts that monster on
/// the field, then reopens the menu (you're still in the Main Phase).
///
/// Response encoding: `[command, index]` — command `1` = summon, `index` = hand slot.
#[test]
fn summoning_from_the_menu_puts_a_monster_on_the_field() {
    let mut duel = Duel::new();

    // Player 0 has one monster in hand.
    duel.add_to_deck(PLAYER_0, Card);
    let monster = duel.draw(PLAYER_0, 1)[0];
    assert_eq!(duel.zone_of(monster), Some(Zone::Hand));

    // Open player 0's Main-Phase menu; it freezes for a choice.
    duel.idle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);

    // Answer: summon the card at hand slot 0.
    duel.set_response(&[CMD_SUMMON, 0]);
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "after summoning, the menu reopens — still the Main Phase"
    );

    assert_eq!(
        duel.zone_of(monster),
        Some(Zone::MonsterZone),
        "the monster is now on the field"
    );
    assert_eq!(duel.hand_count(PLAYER_0), 0, "and gone from the hand");
}
