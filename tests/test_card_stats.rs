//! B0 — card stats (the printed record).
//!
//! A card's `.lua` script declares its whole record (`Card:new(code, { type,
//! atk, def, level, attribute, race, text })`); the engine harvests it keyed by
//! code. When we make a real instance of that code and put it on the field, the
//! instance reports those numbers — the base a Battle Phase needs to compare
//! attacker vs defender.

use std::assert_eq;

use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::PLAYER_0;

// Mirror the prelude's bitmask constants (EDOPro-faithful) for assertions.
const TYPE_MONSTER: u32 = 0x1;
const TYPE_NORMAL: u32 = 0x10;
const ATTRIBUTE_EARTH: u32 = 0x01;
const RACE_BEASTWARRIOR: u64 = 0x8000;

/// Done-when for B0: a summoned monster reports the record its script declared.
#[test]
fn a_summoned_monster_reports_its_stats() {
    let mut duel = Duel::new();
    duel.load_card("cards/BeaverWarrior.lua")
        .expect("BeaverWarrior.lua should load");

    // Make a real instance of that code — its record comes from the loaded script.
    let beaver = duel.make_card(32452818);
    let monster = duel.add_to_hand(PLAYER_0, beaver);
    duel.summon(monster);

    assert_eq!(
        duel.zone_of(monster),
        Some(Zone::MonsterZone),
        "on the field"
    );
    assert_eq!(duel.atk_of(monster), Some(1200), "reports its ATK");
    assert_eq!(duel.def_of(monster), Some(1500), "reports its DEF");
    assert_eq!(duel.level_of(monster), Some(4), "reports its level");

    let data = duel.card_data(monster).expect("has a record");
    assert_eq!(data.card_type, TYPE_MONSTER | TYPE_NORMAL, "Normal Monster");
    assert_eq!(data.attribute, ATTRIBUTE_EARTH, "EARTH");
    assert_eq!(data.race, RACE_BEASTWARRIOR, "Beast-Warrior");
    assert_eq!(data.name, "Beaver Warrior", "carries its name");
    assert_eq!(
        data.text,
        "What this creature lacks in size it makes up for in defense when battling in the prairie.",
        "carries its text"
    );
}

/// A code that was never loaded makes a bare card — zeroed stats, not a panic.
#[test]
fn an_unknown_code_makes_a_bare_card() {
    let duel = Duel::new();
    let ghost = duel.make_card(99999999);
    assert_eq!(ghost.data.atk, 0);
    assert_eq!(ghost.data.card_type, 0);
    assert_eq!(ghost.data.text, "");
}
