//! Event details, rung 1: each raised event carries a **detail bag** (key → value)
//! that a triggered effect can query with `e:get_event_detail(event_code, key)` —
//! e.g. the destroyed card of an `EVENT_DESTROYED`, the summoned monster of an
//! `EVENT_SPECIAL_SUMMON`, the drawn cards of an `EVENT_DRAW`, and so on. The event
//! code is a guard: the value comes back only if the *current* event matches it.
//!
//! ASSUMED API (this test defines the contract — implement to match):
//! - `EVENT_DESTROYED` carries `"destroyed_card"` = the destroyed card.
//! - the event is snapshotted onto the chain link and restored into the context
//!   when the trigger resolves, so verbs can read it.
//! - `e:get_event_detail(code, key)` verb — the detail `key` of the current event
//!   if its code is `code`, else nil.

use cardcrusher::duel::Duel;
use cardcrusher::reason::REASON_EFFECT;
use cardcrusher::zone::Zone;
use cardcrusher::PLAYER_0;

const EVENT_READER: u32 = 90000017;
const EVENT_READER_OPTIONAL: u32 = 90000018;

#[test]
fn a_trigger_reads_the_destroyed_card_from_the_event() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/EventReader.lua")
        .expect("EventReader.lua should load");

    let c = duel.make_card(EVENT_READER);
    let c = duel.add_card(c);
    duel.place(PLAYER_0, c, Zone::MonsterZone);

    // Destroy it → EVENT_DESTROYED fires (with "destroyed_card" = c) → its trigger reads that
    // card from the event and banishes exactly it (GY → Banishment).
    duel.destroy(c, REASON_EFFECT);
    duel.process_events();
    duel.resolve_chain();

    assert_eq!(
        duel.zone_of(c),
        Some(Zone::Banishment),
        "the trigger read the destroyed card from the event and banished it",
    );
}

/// An OPTIONAL (yes/no) trigger reads the event details too: say yes → it banishes
/// the destroyed card the event reports.
#[test]
fn an_optional_trigger_reads_the_event_too() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/EventReaderOptional.lua")
        .expect("EventReaderOptional.lua should load");

    let c = duel.make_card(EVENT_READER_OPTIONAL);
    let c = duel.add_card(c);
    duel.place(PLAYER_0, c, Zone::MonsterZone);

    duel.destroy(c, REASON_EFFECT);
    duel.process_events(); // queues the optional (yes/no) trigger
    duel.process(); // → MSG_SELECT_YESNO
    duel.set_response(&[1]); // yes
    duel.process(); // resolve

    assert_eq!(
        duel.zone_of(c),
        Some(Zone::Banishment),
        "the optional trigger read the event and banished the destroyed card",
    );
}
