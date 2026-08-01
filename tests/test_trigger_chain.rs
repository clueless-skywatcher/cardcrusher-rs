//! Chain engine, C4: a fired TRIGGER builds a chain link and resolves through the
//! chain machinery (response window + LIFO) — instead of resolving inline. This is
//! EDOPro's `process_instant_event` → `PointEvent` → `AddChain` → `QuickEffect` →
//! `SolveChain`, reusing everything C0–C2 already built.
//!
//! Pins the single-mandatory-trigger case: destroying Retaliator (a mandatory
//! `EVENT_DESTROYED` trigger that wipes the opponent) puts its effect ON THE CHAIN
//! — a response window opens, and the wipe only happens once the window is passed.
//! (Contract from `test_chain_response`: `MSG_SELECT_CHAIN`, `CMD_PASS`.)

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::reason::REASON_EFFECT;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_PASS, MSG_SELECT_CHAIN, PLAYER_0, PLAYER_1};

#[test]
fn a_mandatory_trigger_goes_on_the_chain_and_is_respondable() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Retaliator.lua")
        .expect("Retaliator.lua should load");

    // Retaliator (P0) with a mandatory "when destroyed, wipe opponent" trigger;
    // P1 has a monster the trigger will eventually wipe.
    let retaliator = duel.make_card(90000003);
    let retaliator = duel.add_card(retaliator);
    duel.place(PLAYER_0, retaliator, Zone::MonsterZone);
    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);

    // Destroy Retaliator → its trigger fires and is placed on the chain.
    duel.destroy(retaliator, REASON_EFFECT);
    duel.process_events();

    // The trigger is ON THE CHAIN: a response window opens, and it has NOT resolved.
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "a response window opens"
    );
    assert_eq!(*duel.messages().last().unwrap(), MSG_SELECT_CHAIN);
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "the trigger has not resolved yet — it's on the chain, awaiting responses",
    );

    // Pass every window → the trigger resolves through the chain. (A trigger has
    // no menu to return to, so we drive on the `process()` status: keep passing
    // while a window keeps freezing; stop once the chain resolves and the stack
    // empties.)
    duel.set_response(&[CMD_PASS]);
    while duel.process() == DuelStatus::Awaiting {
        duel.set_response(&[CMD_PASS]);
    }
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "passing resolved the trigger (wiped the opponent) via the chain",
    );
}
