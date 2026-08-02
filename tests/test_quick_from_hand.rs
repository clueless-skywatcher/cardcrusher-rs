//! The full quick-from-hand path (the Kuriboh line), end to end: at the opponent's
//! damage calculation, a QUICK effect in hand is offered, activated via the response
//! window, pays its discard cost, chains, resolves, and negates the battle damage —
//! then expires when the battle ends. Also covers the `current_player()` verb (its
//! condition uses it).

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::position::Position;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ATTACK, CMD_PASS, CMD_RESPONSE, MSG_SELECT_CHAIN, PLAYER_0, PLAYER_1};

const HAND_NEGATE: u32 = 90000016;

#[test]
fn a_quick_hand_effect_negates_battle_damage_then_expires() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/HandNegate.lua")
        .expect("HandNegate.lua should load");

    // P1 (the defender) holds the quick effect.
    let guard = duel.make_card(HAND_NEGATE);
    let guard = duel.add_to_hand(PLAYER_1, guard);

    // P0 (turn player) attacks P1 directly for 1500.
    let attacker = duel.add_card(Card::with_data(
        1,
        CardData {
            atk: 1500,
            ..Default::default()
        },
    ));
    duel.place(PLAYER_0, attacker, Zone::MonsterZone);
    duel.change_position(attacker, Position::FaceUpAttack);

    duel.battle_command();
    duel.process(); // MSG_SELECT_BATTLECMD
    duel.set_response(&[CMD_ATTACK, 0]);
    duel.process(); // → damage-calc window

    assert_eq!(
        *duel.messages().last().unwrap(),
        MSG_SELECT_CHAIN,
        "damage window opened"
    );
    let idx = duel
        .chain_response_options()
        .iter()
        .position(|&(c, _)| c == guard)
        .expect("the quick effect is offered in the window");

    // P1 activates it; then everyone passes the remaining windows.
    duel.set_response(&[CMD_RESPONSE, idx as u8]);
    duel.process();
    while duel.messages().last() == Some(&MSG_SELECT_CHAIN) {
        duel.set_response(&[CMD_PASS]);
        if duel.process() == DuelStatus::End {
            break;
        }
    }

    assert_eq!(
        duel.life_points(PLAYER_1),
        8000,
        "the quick effect resolved and negated the 1500 battle damage"
    );
    assert_eq!(
        duel.zone_of(guard),
        Some(Zone::GY),
        "it was discarded as the activation cost"
    );
    assert!(
        duel.can_take_battle_damage(PLAYER_1),
        "the battle ended → the modifier expired",
    );
}
