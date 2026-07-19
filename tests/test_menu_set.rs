//! Acting from the Main-Phase menu: setting a spell/trap.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_SET_SPELL_TRAP, PLAYER_0};

/// Answering the menu with "set the card at hand slot 0" moves it to the
/// Spell/Trap zone, then reopens the menu (still the Main Phase).
///
/// Response encoding: `[command, index]` — command `9` = set spell/trap.
#[test]
fn setting_a_spell_trap_from_the_menu() {
    let mut duel = Duel::new();

    // Player 0 has a card in hand.
    duel.add_to_deck(PLAYER_0, Card);
    let card = duel.draw(PLAYER_0, 1)[0];
    assert_eq!(duel.zone_of(card), Some(Zone::Hand));

    // Open the menu; it freezes for a choice.
    duel.idle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);

    // Answer: set the card at hand slot 0.
    duel.set_response(&[CMD_SET_SPELL_TRAP, 0]);
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "after setting, the menu reopens"
    );

    assert_eq!(
        duel.zone_of(card),
        Some(Zone::SpellTrapZone),
        "the card is now set in the spell/trap zone"
    );
    assert_eq!(duel.hand_count(PLAYER_0), 0, "and gone from the hand");
}
