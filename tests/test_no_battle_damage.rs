//! Kuriboh's core effect via the modifier system: a `NoBattleDamage` **player**
//! modifier — "you take no battle damage." It's player-scoped (not on a card)
//! because Kuriboh works on a *direct attack*, where there is no defending monster
//! to attach it to — mirroring EDOPro's `is_player_affected_by_effect(pd,
//! EFFECT_AVOID_BATTLE_DAMAGE)`.
//!
//! ASSUMED API (this test defines the contract — implement to match):
//! - `duel.add_player_modifier(player: usize, source: CardId, kind: ModifierType)` —
//!   a per-player modifier list (parallel to the per-card one), tagged with source.
//! - `duel.can_take_battle_damage(player) -> bool` — false while a `NoBattleDamage`
//!   modifier is in force for that player.
//! - the damage step zeroes a player's battle damage when the gate is false.
//! - `duel.remove_modifiers_from(source)` clears player modifiers too, not just card
//!   ones.

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::ids::CardId;
use cardcrusher::modifiers::ModifierType;
use cardcrusher::position::Position;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ATTACK, PLAYER_1};

/// A bare card to stand in as the modifier's source (e.g. the Kuriboh instance).
fn bare(duel: &mut Duel) -> CardId {
    duel.add_card(Card::new(0))
}

/// The gate flips with the modifier, and its source can take it back off.
#[test]
fn the_gate_flips_with_the_modifier() {
    let mut duel = Duel::new();
    let src = bare(&mut duel);

    assert!(
        duel.can_take_battle_damage(PLAYER_1),
        "no modifier → can take damage"
    );

    duel.add_player_modifier(PLAYER_1, src, ModifierType::NoBattleDamage);
    assert!(
        !duel.can_take_battle_damage(PLAYER_1),
        "NoBattleDamage in force → protected"
    );

    duel.remove_modifiers_from(src);
    assert!(
        duel.can_take_battle_damage(PLAYER_1),
        "source removed → can take damage again"
    );
}

/// The damage step honors it: a protected player takes no battle damage.
#[test]
fn a_protected_player_takes_no_battle_damage() {
    let mut duel = Duel::new();
    let src = bare(&mut duel);
    duel.add_player_modifier(PLAYER_1, src, ModifierType::NoBattleDamage);

    // P0 attacks P1 directly for 1500 — but P1 is protected.
    let attacker = duel.add_card(Card::with_data(
        1,
        CardData {
            atk: 1500,
            ..Default::default()
        },
    ));
    duel.place(cardcrusher::PLAYER_0, attacker, Zone::MonsterZone);
    duel.change_position(attacker, Position::FaceUpAttack);

    duel.battle_command();
    duel.process();
    duel.set_response(&[CMD_ATTACK, 0]);
    duel.process();

    assert_eq!(
        duel.life_points(PLAYER_1),
        8000,
        "the NoBattleDamage modifier zeroed the battle damage"
    );
}

/// Once the source's modifier is removed, damage lands normally again.
#[test]
fn removing_the_modifier_restores_battle_damage() {
    let mut duel = Duel::new();
    let src = bare(&mut duel);
    duel.add_player_modifier(PLAYER_1, src, ModifierType::NoBattleDamage);
    duel.remove_modifiers_from(src);

    let attacker = duel.add_card(Card::with_data(
        1,
        CardData {
            atk: 1500,
            ..Default::default()
        },
    ));
    duel.place(cardcrusher::PLAYER_0, attacker, Zone::MonsterZone);
    duel.change_position(attacker, Position::FaceUpAttack);

    duel.battle_command();
    duel.process();
    duel.set_response(&[CMD_ATTACK, 0]);
    duel.process();

    assert_eq!(
        duel.life_points(PLAYER_1),
        8000 - 1500,
        "no protection left → full 1500 damage"
    );
}
