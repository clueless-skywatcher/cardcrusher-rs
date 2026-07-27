//! Effect descriptor: an effect advertises **what it does** (a category bitmask)
//! alongside **how it acts** (its kind). Categories ride on the effect's Lua
//! table just like `kind`, harvested via `add_effect(kind, category)`.

use cardcrusher::duel::Duel;

// Mirror the prelude constants (see src/duel/prelude/*.lua) for assertions.
const TRIGGER: u32 = 3;
const EFF_CAT_DESTROY: u32 = 0x1;

/// Avenger's lone effect is a TRIGGER that advertises `EFF_CAT_DESTROY`.
#[test]
fn a_trigger_effect_advertises_its_category() {
    let mut duel = Duel::new();
    duel.load_card("cards/Avenger.lua")
        .expect("Avenger.lua should load");

    let effects = duel.code_effects(90000001);
    assert_eq!(effects.len(), 1, "Avenger registers one effect");

    let effect = &effects[0];
    assert_eq!(
        effect.get::<u32>("kind").expect("kind field"),
        TRIGGER,
        "it acts as a TRIGGER",
    );
    assert_eq!(
        effect.get::<Vec<u32>>("category").expect("category field"),
        vec![EFF_CAT_DESTROY],
        "it advertises that it destroys",
    );
}

/// An effect with no category declared defaults to 0 (advertises nothing).
/// CantActivate's effect is `add_effect(ACTIVATE)` — no category argument.
#[test]
fn an_effect_without_a_category_returns_empty_vector() {
    let mut duel = Duel::new();
    duel.load_card("cards/CantActivate.lua")
        .expect("CantActivate.lua should load");

    let effects = duel.code_effects(11111111);
    assert_eq!(effects.len(), 1);
    assert_eq!(
        effects[0].get::<Vec<u32>>("category").unwrap_or(vec![]),
        vec![],
        "no category declared → 0",
    );
}
