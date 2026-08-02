//! A card grants a PLAYER modifier from Lua: `e:add_player_modifier(who, mod_code)`.
//! Here `NoBattleDamage` — the controller takes no battle damage.
//!
//! ASSUMED API (this test defines the contract — implement to match):
//! - Lua constants `MOD_NO_BATTLE_DAMAGE` (and `MOD_ATK_CHANGE` / `MOD_DEF_CHANGE` /
//!   `MOD_SET_ATK`) — codes bridging `ModifierType` across to Lua.
//! - `e:add_player_modifier(who, mod_code)` verb — `who` is YOU/OPPONENT relative to
//!   the activator; grants that player the modifier, sourced to the effect's card.

use cardcrusher::duel::Duel;
use cardcrusher::PLAYER_0;

const GRANT_NO_DAMAGE: u32 = 90000014;

#[test]
fn a_card_grants_its_controller_no_battle_damage() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/GrantNoDamage.lua")
        .expect("GrantNoDamage.lua should load");

    assert!(
        duel.can_take_battle_damage(PLAYER_0),
        "unprotected to start"
    );

    let spell = duel.make_card(GRANT_NO_DAMAGE);
    let spell = duel.add_to_hand(PLAYER_0, spell);
    let _ = duel.activate(spell, 0, PLAYER_0);
    duel.resolve_chain();

    assert!(
        !duel.can_take_battle_damage(PLAYER_0),
        "resolve granted a NoBattleDamage player modifier → protected",
    );
}
