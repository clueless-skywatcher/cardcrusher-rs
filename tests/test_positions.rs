//! B1 — battle positions.
//!
//! A monster on the field has a position: attack vs defense, face-up vs
//! face-down. Summon → face-up attack; Set → face-down defense; and you can
//! flip / switch it. A card that isn't on the field has no battle position.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::position::Position;
use cardcrusher::zone::Zone;

/// A normally-summoned monster is face-up in attack position.
#[test]
fn a_summoned_monster_is_face_up_attack() {
    let mut duel = Duel::new();
    let m = duel.add_card(Card::new(0));
    duel.summon(m);
    assert_eq!(duel.position_of(m), Some(Position::FaceUpAttack));
}

/// A set monster is face-down in defense position.
#[test]
fn a_set_monster_is_face_down_defense() {
    let mut duel = Duel::new();
    let m = duel.add_card(Card::new(0));
    duel.set_monster(m);

    assert_eq!(duel.position_of(m), Some(Position::FaceDownDefense));
    let pos = duel.position_of(m).unwrap();
    assert!(pos.is_defense() && pos.is_face_down());
}

/// You can flip a set monster face-up, then switch it to defense.
#[test]
fn you_can_change_a_monsters_position() {
    let mut duel = Duel::new();
    let m = duel.add_card(Card::new(0));
    duel.set_monster(m); // face-down defense

    // Flip summon it: face-up attack.
    duel.change_position(m, Position::FaceUpAttack);
    assert_eq!(duel.position_of(m), Some(Position::FaceUpAttack));

    // Switch to face-up defense.
    duel.change_position(m, Position::FaceUpDefense);
    assert_eq!(duel.position_of(m), Some(Position::FaceUpDefense));
}

/// A card that isn't in a Monster Zone has no battle position.
#[test]
fn a_card_off_the_field_has_no_position() {
    let mut duel = Duel::new();
    let c = duel.add_card(Card::new(0));
    assert_eq!(duel.position_of(c), None, "in the arena, not on the field");

    duel.summon(c);
    assert_eq!(duel.position_of(c), Some(Position::FaceUpAttack));

    duel.send_to(c, Zone::GY);
    assert_eq!(duel.position_of(c), None, "in the GY, no position");
}
