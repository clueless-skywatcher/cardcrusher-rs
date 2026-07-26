//! B5 — one attack per monster per turn.
//!
//! Base rule: a monster may declare **one attack per turn** (per Battle Phase).
//! Once it has attacked it's no longer offered as an attacker; the limit resets
//! at the start of each new turn. Enforced at the single chokepoint `attackers`,
//! so the Battle-Phase menu (and board selection) can't offer a spent monster.

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::position::Position;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ATTACK, CMD_NEXT_PHASE, MSG_SELECT_ATTACK_TARGET, MSG_SELECT_BATTLECMD};
use cardcrusher::{PLAYER_0, PLAYER_1};

/// After a monster attacks, it drops out of `attackers` for the rest of the turn.
#[test]
fn a_monster_cannot_attack_twice_in_one_turn() {
    let mut duel = Duel::new();
    let attacker = duel.add_card(Card::new(0));
    duel.summon(attacker); // face-up attack position, controller 0

    assert!(
        duel.attackers(PLAYER_0).contains(&attacker),
        "a fresh monster can attack"
    );

    duel.declare_attack(attacker, None); // spends its one attack this turn

    assert!(
        !duel.attackers(PLAYER_0).contains(&attacker),
        "a monster that already attacked is no longer offered as an attacker"
    );
}

/// The limit resets each turn. Over three turns (p0, p1, p0), player 0's lone
/// 1000-ATK monster attacks directly once per turn — twice total = 2000 damage.
/// Without a per-turn reset it would attack only once (1000 damage).
#[test]
fn the_attack_limit_resets_each_turn() {
    let mut duel = Duel::new();
    duel.set_max_turns(3); // p0, p1, p0

    // Decks so nobody decks out before turn 3.
    for _ in 0..10 {
        duel.add_to_deck(PLAYER_0, Card::new(0));
        duel.add_to_deck(PLAYER_1, Card::new(0));
    }
    // Player 0 controls one 1000-ATK monster; player 1 controls none, so every
    // attack is a direct attack (no target-selection step to drive).
    let attacker = duel.add_card(Card::with_data(
        1,
        CardData {
            atk: 1000,
            ..Default::default()
        },
    ));
    duel.place(PLAYER_0, attacker, Zone::MonsterZone);
    duel.change_position(attacker, Position::FaceUpAttack);

    duel.start();

    // Drive: in each Battle Phase try one attack, then move on; skip other menus.
    // The toggle guarantees forward progress (the second answer always advances).
    let mut tried_attack = false;
    loop {
        match duel.process() {
            DuelStatus::End => break,
            DuelStatus::Awaiting => match duel.messages().last().copied() {
                Some(MSG_SELECT_BATTLECMD) => {
                    if tried_attack {
                        duel.set_response(&[CMD_NEXT_PHASE]);
                        tried_attack = false;
                    } else {
                        duel.set_response(&[CMD_ATTACK, 0]);
                        tried_attack = true;
                    }
                }
                Some(MSG_SELECT_ATTACK_TARGET) => duel.set_response(&[0]),
                _ => duel.set_response(&[CMD_NEXT_PHASE]), // Main-Phase menus
            },
            DuelStatus::Continue => unreachable!("process runs until End or Awaiting"),
        }
    }

    assert_eq!(
        duel.life_points(PLAYER_1),
        8000 - 2 * 1000,
        "one attack per turn × two of player 0's turns = two direct hits"
    );
}
