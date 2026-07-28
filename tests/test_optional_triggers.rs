//! Optional triggers ("you can"): a fired optional trigger asks its controller
//! before resolving — yes resolves, no does nothing. Mandatory triggers (the
//! existing `test_triggers` cases) resolve without asking and stay green.
//!
//! Proposed surface (what these tests pin):
//! - A trigger effect may set `optional = true` on its Lua table.
//! - When such a trigger fires, the duel freezes on `MSG_SELECT_YESNO`.
//! - Response `[1]` = activate, `[0]` = decline.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::reason::REASON_EFFECT;
use cardcrusher::zone::Zone;
use cardcrusher::{MSG_SELECT_YESNO, PLAYER_0, PLAYER_1};

/// Put the Optional Avenger under P0, plus an opponent monster to wipe. Returns
/// (optional-avenger id, foe id).
fn setup() -> (Duel, cardcrusher::ids::CardId, cardcrusher::ids::CardId) {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/OptionalAvenger.lua")
        .expect("OptionalAvenger.lua should load");
    let opt = duel.make_card(90000005);
    let opt = duel.add_card(opt);
    duel.place(PLAYER_0, opt, Zone::MonsterZone);

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    (duel, opt, foe)
}

/// The optional trigger fires → the duel freezes asking yes/no → answering **yes**
/// resolves it (opponent's board is wiped).
#[test]
fn optional_trigger_resolves_on_yes() {
    let (mut duel, opt, foe) = setup();

    duel.destroy(opt, REASON_EFFECT);
    duel.process_events(); // queues the optional trigger (does NOT resolve yet)

    assert_eq!(duel.process(), DuelStatus::Awaiting, "freezes to ask");
    assert_eq!(
        *duel.messages().last().unwrap(),
        MSG_SELECT_YESNO,
        "it's a yes/no prompt",
    );
    assert_eq!(duel.zone_of(foe), Some(Zone::MonsterZone), "nothing yet");

    duel.set_response(&[1]); // yes
    duel.process();
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "answering yes resolved the optional trigger",
    );
}

/// Answering **no** declines: nothing happens.
#[test]
fn optional_trigger_declined_does_nothing() {
    let (mut duel, opt, foe) = setup();

    duel.destroy(opt, REASON_EFFECT);
    duel.process_events();
    assert_eq!(duel.process(), DuelStatus::Awaiting, "freezes to ask");

    duel.set_response(&[0]); // no
    duel.process();
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "declining an optional trigger leaves the board untouched",
    );
}
