//! B2 — declaring an attack.
//!
//! In the Battle Phase, a menu offers the monsters you can attack with (your
//! face-up attack-position monsters). You pick an attacker, then pick a target
//! monster — or, if the opponent has none, it's a direct attack. Declaring
//! records "A attacks B" (damage itself comes in B3).

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ATTACK, MSG_SELECT_BATTLECMD, PLAYER_0, PLAYER_1};

/// Only your face-up attack-position monsters can be declared as attackers.
#[test]
fn only_face_up_attack_monsters_can_attack() {
    let mut duel = Duel::new();

    let ready = duel.add_card(Card::new(0));
    duel.summon(ready); // face-up attack — can attack

    let defending = duel.add_card(Card::new(0));
    duel.set_monster(defending); // face-down defense — cannot attack

    assert_eq!(
        duel.attackers(PLAYER_0),
        vec![ready],
        "only the attack-position monster is offered"
    );
}

/// Declaring an attack against a monster records the attacker and that target.
#[test]
fn declaring_an_attack_records_attacker_and_target() {
    let mut duel = Duel::new();

    // p0 has an attack-position monster; p1 has a monster to be attacked.
    let attacker = duel.add_card(Card::new(0));
    duel.summon(attacker); // owner 0, face-up attack

    let target = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, target, Zone::MonsterZone);

    // Open p0's Battle-Phase menu.
    duel.battle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(*duel.messages().last().unwrap(), MSG_SELECT_BATTLECMD);

    // Attack with attacker index 0 → now pick a target.
    duel.set_response(&[CMD_ATTACK, 0]);
    assert_eq!(duel.process(), DuelStatus::Awaiting, "pick a target");

    // Target index 0 → declaration recorded; the Battle menu reopens.
    duel.set_response(&[0]);
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "back to the battle menu"
    );

    assert_eq!(duel.last_attack(), Some((attacker, Some(target))));
}

/// With no opponent monsters, declaring an attack is a direct attack (no target).
#[test]
fn declaring_a_direct_attack_records_no_target() {
    let mut duel = Duel::new();

    let attacker = duel.add_card(Card::new(0));
    duel.summon(attacker); // owner 0, face-up attack; p1 has no monsters

    duel.battle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);

    // Attack index 0 → no targets exist → declared directly, menu reopens.
    duel.set_response(&[CMD_ATTACK, 0]);
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "no target pick needed"
    );

    assert_eq!(duel.last_attack(), Some((attacker, None)), "direct attack");
}
