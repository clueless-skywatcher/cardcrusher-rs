//! Chain engine, rung 2: `spell_speed` derives an effect's speed (0..3) from its
//! kind + the owning card's class/subtype — a pure function, mirroring EDOPro
//! `effect::get_speed()`. Pins the truth table in `docs/chain.md`.

use cardcrusher::effect::{spell_speed, EffectKind};

// Card-type bits (mirror prelude/card_types.lua).
const MONSTER: u32 = 0x1;
const SPELL: u32 = 0x2;
const TRAP: u32 = 0x4;
// Subtype values (mirror spell_types.lua / trap_types.lua).
const SPELL_NORMAL: u32 = 1;
const SPELL_QUICKPLAY: u32 = 2;
const TRAP_NORMAL: u32 = 1;
const TRAP_COUNTER: u32 = 3;

#[test]
fn quick_is_speed_2() {
    assert_eq!(spell_speed(EffectKind::Quick, MONSTER, None, None), 2);
}

#[test]
fn ignition_and_trigger_are_speed_1() {
    assert_eq!(spell_speed(EffectKind::Ignition, MONSTER, None, None), 1);
    assert_eq!(spell_speed(EffectKind::Trigger, MONSTER, None, None), 1);
}

#[test]
fn normal_spell_activation_is_speed_1() {
    assert_eq!(
        spell_speed(EffectKind::Activate, SPELL, Some(SPELL_NORMAL), None),
        1
    );
}

#[test]
fn quick_play_spell_is_speed_2() {
    assert_eq!(
        spell_speed(EffectKind::Activate, SPELL, Some(SPELL_QUICKPLAY), None),
        2
    );
}

#[test]
fn normal_trap_is_speed_2() {
    assert_eq!(
        spell_speed(EffectKind::Activate, TRAP, None, Some(TRAP_NORMAL)),
        2
    );
}

#[test]
fn counter_trap_is_speed_3() {
    assert_eq!(
        spell_speed(EffectKind::Activate, TRAP, None, Some(TRAP_COUNTER)),
        3
    );
}

#[test]
fn monster_activation_is_speed_0() {
    // Shouldn't occur (monster effects are IGNITION/TRIGGER), but the derive is total.
    assert_eq!(spell_speed(EffectKind::Activate, MONSTER, None, None), 0);
}
