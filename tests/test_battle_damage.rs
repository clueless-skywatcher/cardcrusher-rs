//! B3 — battle damage (monster vs monster, and direct).
//!
//! Resolving a declared attack compares the attacker's ATK against the target's
//! ATK (if the target is in attack position) or DEF (if in defense position):
//! the higher wins, the loser goes to the GY, and the *difference* is dealt as
//! life-point damage — but only against an attack-position monster. A direct
//! attack (no target) deals the attacker's full ATK to the opponent.

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::position::Position;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Put a monster with the given ATK/DEF on `owner`'s field in `pos`.
fn monster(
    duel: &mut Duel,
    owner: usize,
    atk: i32,
    def: i32,
    pos: Position,
) -> cardcrusher::ids::CardId {
    let c = duel.add_card(Card::with_data(
        1,
        CardData {
            atk,
            def,
            ..Default::default()
        },
    ));
    duel.place(owner, c, Zone::MonsterZone);
    duel.change_position(c, pos);
    c
}

// ===== Target in ATTACK position (ATK vs ATK) ==============================

#[test]
fn stronger_attacker_destroys_target_and_deals_the_difference() {
    let mut duel = Duel::new();
    let attacker = monster(&mut duel, PLAYER_0, 1800, 0, Position::FaceUpAttack);
    let target = monster(&mut duel, PLAYER_1, 1200, 0, Position::FaceUpAttack);

    duel.resolve_battle(attacker, Some(target));

    assert_eq!(duel.zone_of(target), Some(Zone::GY), "target destroyed");
    assert_eq!(
        duel.zone_of(attacker),
        Some(Zone::MonsterZone),
        "attacker lives"
    );
    assert_eq!(duel.life_points(PLAYER_1), 8000 - 600, "1800-1200 = 600");
}

#[test]
fn weaker_attacker_dies_and_its_controller_takes_the_difference() {
    let mut duel = Duel::new();
    let attacker = monster(&mut duel, PLAYER_0, 1000, 0, Position::FaceUpAttack);
    let target = monster(&mut duel, PLAYER_1, 1500, 0, Position::FaceUpAttack);

    duel.resolve_battle(attacker, Some(target));

    assert_eq!(duel.zone_of(attacker), Some(Zone::GY), "attacker destroyed");
    assert_eq!(
        duel.zone_of(target),
        Some(Zone::MonsterZone),
        "target lives"
    );
    assert_eq!(duel.life_points(PLAYER_0), 8000 - 500, "1500-1000 = 500");
}

#[test]
fn equal_attack_destroys_both_with_no_damage() {
    let mut duel = Duel::new();
    let attacker = monster(&mut duel, PLAYER_0, 1500, 0, Position::FaceUpAttack);
    let target = monster(&mut duel, PLAYER_1, 1500, 0, Position::FaceUpAttack);

    duel.resolve_battle(attacker, Some(target));

    assert_eq!(duel.zone_of(attacker), Some(Zone::GY), "both destroyed");
    assert_eq!(duel.zone_of(target), Some(Zone::GY), "both destroyed");
    assert_eq!(duel.life_points(PLAYER_0), 8000, "no damage");
    assert_eq!(duel.life_points(PLAYER_1), 8000, "no damage");
}

// ===== Target in DEFENSE position (ATK vs DEF) =============================

#[test]
fn attack_beats_defense_destroys_target_no_damage() {
    let mut duel = Duel::new();
    let attacker = monster(&mut duel, PLAYER_0, 1800, 0, Position::FaceUpAttack);
    let target = monster(&mut duel, PLAYER_1, 0, 1500, Position::FaceUpDefense);

    duel.resolve_battle(attacker, Some(target));

    assert_eq!(duel.zone_of(target), Some(Zone::GY), "1800 > 1500 DEF");
    assert_eq!(duel.life_points(PLAYER_1), 8000, "no damage vs defense");
}

#[test]
fn attack_into_bigger_defense_bounces_and_hurts_attacker() {
    let mut duel = Duel::new();
    let attacker = monster(&mut duel, PLAYER_0, 1000, 0, Position::FaceUpAttack);
    let target = monster(&mut duel, PLAYER_1, 0, 1500, Position::FaceUpDefense);

    duel.resolve_battle(attacker, Some(target));

    assert_eq!(
        duel.zone_of(attacker),
        Some(Zone::MonsterZone),
        "attacker survives"
    );
    assert_eq!(
        duel.zone_of(target),
        Some(Zone::MonsterZone),
        "target survives"
    );
    assert_eq!(
        duel.life_points(PLAYER_0),
        8000 - 500,
        "1500-1000 = 500 to attacker"
    );
}

#[test]
fn attack_equals_defense_does_nothing() {
    let mut duel = Duel::new();
    let attacker = monster(&mut duel, PLAYER_0, 1500, 0, Position::FaceUpAttack);
    let target = monster(&mut duel, PLAYER_1, 0, 1500, Position::FaceUpDefense);

    duel.resolve_battle(attacker, Some(target));

    assert_eq!(duel.zone_of(attacker), Some(Zone::MonsterZone));
    assert_eq!(duel.zone_of(target), Some(Zone::MonsterZone));
    assert_eq!(duel.life_points(PLAYER_0), 8000);
    assert_eq!(duel.life_points(PLAYER_1), 8000);
}

// ===== Direct attack =======================================================

#[test]
fn a_direct_attack_deals_full_atk() {
    let mut duel = Duel::new();
    let attacker = monster(&mut duel, PLAYER_0, 1800, 0, Position::FaceUpAttack);

    duel.resolve_battle(attacker, None);

    assert_eq!(
        duel.life_points(PLAYER_1),
        8000 - 1800,
        "full ATK to the opponent"
    );
}
