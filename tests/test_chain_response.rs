//! Chain engine, rung 5: the response window (`ChainResponse`). After an
//! activation, players get windows to respond; two consecutive passes close the
//! window and the chain resolves LIFO.
//!
//! ASSUMED CONTRACT (these tests DEFINE it — implement rung 5 to match, or tell me
//! to adjust):
//! - `MSG_SELECT_CHAIN` is emitted whenever a response window is open.
//! - Windows open **opponent-first**, then ping-pong; two consecutive passes (no
//!   activation between) close the window → resolve LIFO. (Whether an *empty*
//!   window is auto-passed or still shown is left open — the tests loop-pass every
//!   window, so either policy works.)
//! - Response bytes: `[CMD_PASS]` = **pass**; `[CMD_RESPONSE, i]` = **respond** by
//!   activating `chainable_effects(player)[i]`.
//! - Entry: activating from the Main-Phase menu (`CMD_ACTIVATE`) puts link 1 on the
//!   chain and opens the opponent's window.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ACTIVATE, CMD_PASS, CMD_RESPONSE, MSG_SELECT_CHAIN, PLAYER_0, PLAYER_1};

const PASS: &[u8] = &[CMD_PASS];

/// Pass every open response window until none remain.
fn pass_all_windows(duel: &mut Duel) {
    while duel.messages().last() == Some(&MSG_SELECT_CHAIN) {
        duel.set_response(PASS);
        duel.process();
    }
}

/// C1 — a lone activation: the opponent's window opens, and passing (through every
/// window) resolves the chain.
#[test]
fn passing_the_response_window_resolves_the_chain() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Nuke.lua")
        .expect("Nuke.lua should load");
    duel.load_card("tests/fixtures/QuickNuke.lua")
        .expect("QuickNuke.lua should load");

    let nuke = duel.make_card(90000006);
    duel.add_to_hand(PLAYER_0, nuke);
    // P1 holds a quick-play so their window definitely opens (robust to any
    // empty-window policy), plus a monster for Nuke to wipe.
    let quick = duel.make_card(90000008);
    duel.add_to_hand(PLAYER_1, quick);
    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);

    // P0 activates Nuke from the Main-Phase menu → link 1 + response window.
    duel.idle_command();
    duel.process(); // → MSG_SELECT_IDLECMD
    duel.set_response(&[CMD_ACTIVATE, 0]);
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "a response window opens"
    );
    assert_eq!(*duel.messages().last().unwrap(), MSG_SELECT_CHAIN);
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "nothing has resolved yet",
    );

    pass_all_windows(&mut duel);
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "passing every window resolved the chain",
    );
}

/// C2 — a response chains a 2nd link, and the chain resolves LIFO. P1 chains a
/// quick bounce of its own monster: because link 2 resolves FIRST, the monster is
/// off the field before Nuke re-reads it → it survives (in hand) instead of the GY.
#[test]
fn a_chained_response_resolves_before_what_it_chained_to() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Nuke.lua")
        .expect("Nuke.lua should load");
    duel.load_card("tests/fixtures/QuickRetreat.lua")
        .expect("QuickRetreat.lua should load");

    let nuke = duel.make_card(90000006);
    duel.add_to_hand(PLAYER_0, nuke);
    let mine = duel.add_card(Card::new(0)); // P1's monster
    duel.place(PLAYER_1, mine, Zone::MonsterZone);
    let retreat = duel.make_card(90000009);
    duel.add_to_hand(PLAYER_1, retreat);

    // P0 activates Nuke (link 1) → P1's response window.
    duel.idle_command();
    duel.process();
    duel.set_response(&[CMD_ACTIVATE, 0]);
    duel.process();
    assert_eq!(*duel.messages().last().unwrap(), MSG_SELECT_CHAIN);

    // P1 responds with Quick Retreat (chainable index 0) → link 2.
    duel.set_response(&[CMD_RESPONSE, 0]);
    duel.process();

    // Everyone passes from here → resolve LIFO (link 2 first, then link 1).
    pass_all_windows(&mut duel);

    assert_eq!(
        duel.zone_of(mine),
        Some(Zone::Hand),
        "LIFO: Quick Retreat resolved first (monster → hand), so Nuke found nothing",
    );
}
