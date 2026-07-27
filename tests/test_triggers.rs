//! Event/trigger foundation (E1): a `TRIGGER` effect fires off an *event*, not an
//! activation. Start with destruction-by-battle.
//!
//! Model under test:
//! - `destroy(card, reason)` queues a "destroyed" event.
//! - `process_events()` drains the queue and, for a **battle** destruction, fires
//!   that card's `TRIGGER` effects (with the card's controller as the activator,
//!   so `OPPONENT` resolves correctly even though the card is now in the GY).
//!
//! Avenger: "If this card is destroyed by battle: destroy all monsters your
//! opponent controls." ("If" ⇒ no miss-the-timing ⇒ fires on the event.)

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::reason::{REASON_BATTLE, REASON_EFFECT};
use cardcrusher::zone::Zone;
use cardcrusher::{
    CMD_ACTIVATE, CMD_ATTACK, MSG_SELECT_ATTACK_TARGET, MSG_SELECT_BATTLECMD, PLAYER_0, PLAYER_1,
};

/// Put the Avenger under `player`'s control on the field, and return its id.
fn place_avenger(duel: &mut Duel, player: usize) -> cardcrusher::ids::CardId {
    duel.load_card("cards/Avenger.lua")
        .expect("Avenger.lua should load");
    let avenger = duel.make_card(90000001);
    let id = duel.add_card(avenger);
    duel.place(player, id, Zone::MonsterZone);
    id
}

/// Destroyed **by battle**, the Avenger's trigger fires and wipes the opponent's
/// monsters.
#[test]
fn if_trigger_fires_when_destroyed_by_battle() {
    let mut duel = Duel::new();
    let avenger = place_avenger(&mut duel, PLAYER_0);

    // Two plain monsters the opponent (P1) controls.
    let foe1 = duel.add_card(Card::new(0));
    let foe2 = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe1, Zone::MonsterZone);
    duel.place(PLAYER_1, foe2, Zone::MonsterZone);

    duel.destroy(avenger, REASON_BATTLE);
    duel.process_events();

    assert_eq!(duel.zone_of(avenger), Some(Zone::GY), "the Avenger is gone");
    assert_eq!(duel.zone_of(foe1), Some(Zone::GY), "trigger wiped opp monster");
    assert_eq!(duel.zone_of(foe2), Some(Zone::GY), "trigger wiped opp monster");
}

/// Destroyed by an *effect* (not battle), the "…by battle" trigger stays silent.
#[test]
fn trigger_does_not_fire_on_non_battle_destruction() {
    let mut duel = Duel::new();
    let avenger = place_avenger(&mut duel, PLAYER_0);

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);

    duel.destroy(avenger, REASON_EFFECT);
    duel.process_events();

    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "a non-battle destruction must not trigger the battle effect",
    );
}

/// A trigger whose `condition` returns false must NOT fire, even on a battle
/// destruction that would otherwise match. DudTrigger has `condition = false`
/// and the same board-wipe resolve as Avenger, so any firing would be obvious.
#[test]
fn trigger_with_false_condition_does_not_fire() {
    let mut duel = Duel::new();
    duel.load_card("cards/DudTrigger.lua")
        .expect("DudTrigger.lua should load");
    let dud = duel.make_card(90000002);
    let dud = duel.add_card(dud);
    duel.place(PLAYER_0, dud, Zone::MonsterZone);

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);

    duel.destroy(dud, REASON_BATTLE);
    duel.process_events();

    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "a false condition must keep the trigger silent",
    );
}

/// End-to-end through the real Battle Phase: a strong attacker kills the Avenger,
/// whose trigger then wipes the attacker's side — the attacker destroys itself by
/// picking a fight. Exercises whatever wiring drains the event queue after a
/// battle (currently unwired, so this is red until `process_events` is called
/// from the battle flow).
#[test]
fn battle_destruction_fires_trigger_end_to_end() {
    let mut duel = Duel::new();

    // P0 attacks with a 2000-ATK beater; P1 defends with the 1000-ATK Avenger.
    let attacker = duel.add_card(Card::with_data(
        1,
        CardData {
            atk: 2000,
            ..Default::default()
        },
    ));
    duel.place(PLAYER_0, attacker, Zone::MonsterZone);
    let avenger = place_avenger(&mut duel, PLAYER_1);

    // Drive P0's Battle Phase: attack (index 0) → target the Avenger (index 0).
    duel.battle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(*duel.messages().last().unwrap(), MSG_SELECT_BATTLECMD);

    duel.set_response(&[CMD_ATTACK, 0]);
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(*duel.messages().last().unwrap(), MSG_SELECT_ATTACK_TARGET);

    duel.set_response(&[0]);
    duel.process(); // resolve the battle (and, once wired, its triggers)

    assert_eq!(duel.zone_of(avenger), Some(Zone::GY), "Avenger died in battle");
    assert_eq!(
        duel.zone_of(attacker),
        Some(Zone::GY),
        "the Avenger's trigger wiped the attacker in revenge",
    );
    assert_eq!(duel.life_points(PLAYER_1), 8000 - 1000, "2000 vs 1000 = 1000");
}

// ===== Generic dispatch: subscribe by event code, drain covers every path =====

/// Retaliator subscribes to the generic `EVENT_DESTROYED`, so it fires on ANY
/// destruction — here an *effect* destroy, not battle. This forces the engine to
/// dispatch by the event's code (matching the effect's subscription) rather than
/// hardcoding "battle destruction of self".
#[test]
fn any_destroy_trigger_fires_regardless_of_reason() {
    let mut duel = Duel::new();
    duel.load_card("cards/Retaliator.lua")
        .expect("Retaliator.lua should load");
    let retaliator = duel.make_card(90000003);
    let retaliator = duel.add_card(retaliator);
    duel.place(PLAYER_0, retaliator, Zone::MonsterZone);

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);

    duel.destroy(retaliator, REASON_EFFECT); // NOT battle
    duel.process_events();

    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "an EVENT_DESTROYED trigger fires regardless of the destruction's reason",
    );
}

/// The complement of the Avenger split: a *battle* kill is also a destruction, so
/// a generic `EVENT_DESTROYED` trigger (Retaliator) must fire on battle too. This
/// pins that a battle-destroy raises `EVENT_DESTROYED` (not only the battle code),
/// so the code-granularity refactor can't accidentally starve generic triggers.
#[test]
fn a_generic_destroy_trigger_also_fires_on_battle() {
    let mut duel = Duel::new();
    duel.load_card("cards/Retaliator.lua")
        .expect("Retaliator.lua should load");
    let retaliator = duel.make_card(90000003);
    let retaliator = duel.add_card(retaliator);
    duel.place(PLAYER_0, retaliator, Zone::MonsterZone);

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);

    duel.destroy(retaliator, REASON_BATTLE);
    duel.process_events();

    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "a battle destruction is still a destruction — EVENT_DESTROYED fires",
    );
}

/// End-to-end through the Main-Phase menu: an *effect* (Example Spell) destroys a
/// trigger monster, and its trigger fires — with NO manual `process_events`. This
/// pins the second half of the generalization: one central drain covers the
/// effect path too, not just battle.
#[test]
fn an_effect_kill_fires_the_victims_trigger() {
    let mut duel = Duel::new();
    duel.load_card("cards/ExampleSpell.lua")
        .expect("ExampleSpell.lua should load");
    duel.load_card("cards/Retaliator.lua")
        .expect("Retaliator.lua should load");

    // P0 holds Example Spell and has a bystander monster; P1 controls Retaliator.
    duel.add_to_hand(PLAYER_0, Card::new(12345678));
    let bystander = duel.add_card(Card::new(0));
    duel.place(PLAYER_0, bystander, Zone::MonsterZone);
    let retaliator = duel.make_card(90000003);
    let retaliator = duel.add_card(retaliator);
    duel.place(PLAYER_1, retaliator, Zone::MonsterZone);

    // Main-Phase menu: activate Example Spell (slot 0) → target Retaliator (idx 0).
    duel.idle_command();
    duel.process(); // menu
    duel.set_response(&[CMD_ACTIVATE, 0]);
    duel.process(); // → target selection
    duel.set_response(&[0]); // pick Retaliator
    duel.process(); // resolve: destroys Retaliator by effect; drain must fire its trigger

    assert_eq!(
        duel.zone_of(retaliator),
        Some(Zone::GY),
        "Example Spell destroyed Retaliator",
    );
    assert_eq!(
        duel.zone_of(bystander),
        Some(Zone::GY),
        "Retaliator's trigger fired on the effect-kill and wiped P0's board",
    );
}
