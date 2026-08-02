//! Kuriboh rungs 3 & 4: the **damage-calculation window**.
//!
//! Rung 3 — before battle damage is applied, a response window opens; the damage
//! is *pending* (LP untouched) until the window closes.
//! Rung 4 — at that window, a QUICK effect sitting in the responder's HAND (whose
//! timing matches damage calc) is offered, even though the chain is empty.
//!
//! NEW surface assumed:
//! - `resolve_battle` computes pending damage, opens a `ChainResponse` window
//!   (`MSG_SELECT_CHAIN`), and applies the damage only after it closes.
//! - `chain_response_options()` at the damage window collects timing-matched QUICK
//!   effects from the responder's hand (no top link required — the chain is empty).

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::position::Position;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ATTACK, CMD_PASS, MSG_SELECT_CHAIN, PLAYER_0, PLAYER_1};

const PRE_DAMAGE_CALC_EVENT: u32 = 90000011;

/// Give P0 a face-up attacker and open P0's Battle Phase menu.
fn attacker_1500(duel: &mut Duel) -> cardcrusher::ids::CardId {
    let c = duel.add_card(Card::with_data(
        1,
        CardData {
            atk: 1500,
            ..Default::default()
        },
    ));
    duel.place(PLAYER_0, c, Zone::MonsterZone);
    duel.change_position(c, Position::FaceUpAttack);
    c
}

/// Drive P0's Battle Phase to a direct attack; stop at the damage-calc window.
fn declare_direct_attack(duel: &mut Duel) {
    duel.battle_command();
    duel.process(); // MSG_SELECT_BATTLECMD
    duel.set_response(&[CMD_ATTACK, 0]);
    duel.process(); // no target → direct attack → (rung 3) opens the damage window
}

// ===== Rung 3: the window defers the damage =================================

#[test]
fn battle_damage_is_deferred_behind_a_response_window() {
    let mut duel = Duel::new();
    // A window only opens if someone can respond, so give the defender (P1) a
    // matching quick effect in hand.
    duel.load_card("tests/fixtures/PreDamageCalcEvent.lua")
        .expect("PreDamageCalcEvent.lua should load");
    let guard = duel.make_card(PRE_DAMAGE_CALC_EVENT);
    duel.add_to_hand(PLAYER_1, guard);

    attacker_1500(&mut duel);

    declare_direct_attack(&mut duel);

    // A response window opened at damage calc — and the LP hasn't dropped yet.
    assert_eq!(
        *duel.messages().last().unwrap(),
        MSG_SELECT_CHAIN,
        "a response window opens at damage calculation",
    );
    assert_eq!(
        duel.life_points(PLAYER_1),
        8000,
        "the 1500 damage is pending — not applied until the window closes",
    );

    // Everyone passes → the pending damage is finally applied.
    while duel.messages().last() == Some(&MSG_SELECT_CHAIN) {
        duel.set_response(&[CMD_PASS]);
        if duel.process() == DuelStatus::End {
            break;
        }
    }
    assert_eq!(
        duel.life_points(PLAYER_1),
        8000 - 1500,
        "once the window closes, the deferred damage lands",
    );
}

// ===== Rung 4: a quick effect in hand is offered at that window =============

#[test]
fn a_quick_hand_effect_is_offered_at_the_damage_window() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/PreDamageCalcEvent.lua")
        .expect("PreDamageCalcEvent.lua should load");

    // The DEFENDER (P1, the one who'd take the hit) holds the quick effect.
    let guard = duel.make_card(PRE_DAMAGE_CALC_EVENT);
    let guard = duel.add_to_hand(PLAYER_1, guard);

    attacker_1500(&mut duel);
    declare_direct_attack(&mut duel);

    assert_eq!(
        *duel.messages().last().unwrap(),
        MSG_SELECT_CHAIN,
        "the damage window is open",
    );
    assert!(
        duel.chain_response_options()
            .iter()
            .any(|&(card, _)| card == guard),
        "a timing-matched QUICK effect in the defender's hand is offered as a response",
    );
}
