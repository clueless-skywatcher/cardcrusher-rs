//! A granted modifier expires when a battle ends — via `queue` + `remove_modifier`.
//! On resolve: `local passive = add_player_modifier(...)` (the modifier's id), then
//! `queue(EVENT_BATTLE_ENDED, {1, ONCE}, () -> remove_modifier(passive))`.
//!
//! ASSUMED API (this test pins the queue subsystem — implement to match):
//! - `add_player_modifier` returns the new modifier's **id** to Lua.
//! - `e:remove_modifier(id)` verb — remove the one modifier with that id (from any
//!   card or player list).
//! - `e:queue(event, {count, period}, fn)` verb — run `fn` when `event` fires, up to
//!   `count` times within `period`; `ONCE` = exactly once, ever.
//! - `EVENT_BATTLE_ENDED` is raised after each battle resolves, firing subscriptions.

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::position::Position;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ATTACK, PLAYER_0, PLAYER_1};

const TEMP_NO_DAMAGE: u32 = 90000015;

#[test]
fn a_queued_removal_expires_the_modifier_when_the_battle_ends() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/TempNoDamage.lua")
        .expect("TempNoDamage.lua should load");

    // P1 grants itself temporary protection.
    let spell = duel.make_card(TEMP_NO_DAMAGE);
    let spell = duel.add_to_hand(PLAYER_1, spell);
    let _ = duel.activate(spell, 0, PLAYER_1);
    duel.resolve_chain();
    assert!(
        !duel.can_take_battle_damage(PLAYER_1),
        "protected right after resolve"
    );

    // P0 attacks P1 directly — protected, so no damage lands.
    let attacker = duel.add_card(Card::with_data(
        1,
        CardData {
            atk: 1500,
            ..Default::default()
        },
    ));
    duel.place(PLAYER_0, attacker, Zone::MonsterZone);
    duel.change_position(attacker, Position::FaceUpAttack);
    duel.battle_command();
    duel.process();
    duel.set_response(&[CMD_ATTACK, 0]);
    duel.process();

    assert_eq!(
        duel.life_points(PLAYER_1),
        8000,
        "no damage during the protected battle"
    );
    assert!(
        duel.can_take_battle_damage(PLAYER_1),
        "the battle ended → the queued removal fired → protection is gone",
    );
}
