//! M6: activate a card's effect from the Main-Phase menu, driven entirely
//! through the processor (`process()` / `Awaiting` / `set_response`) — the menu,
//! the `Activate` unit, the coroutine freeze/resume, and the DSL all together.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{
    CMD_ACTIVATE, CMD_PASS, MSG_SELECT_CHAIN, MSG_SELECT_IDLECMD, PLAYER_0, PLAYER_1,
};

/// After an activation, the chain opens a response window; pass every window so
/// the chain resolves. Returns the status of the final `process()`.
fn pass_windows(duel: &mut Duel) -> DuelStatus {
    let mut status = DuelStatus::Awaiting;
    while duel.messages().last() == Some(&MSG_SELECT_CHAIN) {
        duel.set_response(&[CMD_PASS]);
        status = duel.process();
    }
    status
}

/// Menu → "activate the card in hand slot 0" → its target freezes for a pick →
/// answer → resolve → the target is destroyed. Response encoding:
/// `[CMD_ACTIVATE, hand_slot]`, then `[candidate_index]` for the selection.
#[test]
fn activating_an_effect_from_the_menu() {
    let mut duel = Duel::new();
    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    duel.load_card("tests/fixtures/ExampleSpell.lua")
        .expect("ExampleSpell.lua should load");
    // Player 0 holds the Example card.
    duel.add_to_hand(PLAYER_0, Card::new(12345678));

    // Open the Main-Phase menu; it freezes for a choice.
    duel.idle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);

    // Choose "activate hand slot 0" → the effect's target freezes for a pick.
    duel.set_response(&[CMD_ACTIVATE, 0]);
    assert_eq!(
        duel.process(),
        DuelStatus::Awaiting,
        "activating should freeze for the effect's target selection"
    );
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "nothing destroyed while still choosing"
    );

    // Pick candidate index 0 → resume → response window opens.
    duel.set_response(&[0]);
    duel.process();
    // Pass the response window(s) → the chain resolves → destroy.
    pass_windows(&mut duel);
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "the activated effect destroyed its target"
    );
}

/// After activating from the menu, you return to the Main-Phase menu — you don't
/// lose control of the phase. (Summon already re-shows the menu; activate must
/// too.) With the standalone menu, the bug shows up as the duel ending early.
#[test]
fn the_menu_reopens_after_activating() {
    let mut duel = Duel::new();
    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    duel.load_card("tests/fixtures/ExampleSpell.lua")
        .expect("ExampleSpell.lua should load");
    duel.add_to_hand(PLAYER_0, Card::new(12345678));

    duel.idle_command();
    duel.process(); // menu

    duel.set_response(&[CMD_ACTIVATE, 0]);
    duel.process(); // → target selection

    duel.set_response(&[0]);
    duel.process(); // → response window opens
    let status = pass_windows(&mut duel); // pass → resolve → should re-open the menu

    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "the effect still resolved"
    );
    assert_eq!(
        status,
        DuelStatus::Awaiting,
        "should return to the Main-Phase menu, not end/advance the phase"
    );
    assert_eq!(
        duel.messages().last().copied(),
        Some(MSG_SELECT_IDLECMD),
        "the menu is shown again"
    );
}

/// A Spell's activation lifecycle mirrors EDOPro: it moves from the hand to the
/// S/T zone when activated (it's on the field while it resolves), then to the
/// graveyard once its effect resolves.
#[test]
fn an_activated_spell_goes_to_field_then_graveyard() {
    let mut duel = Duel::new();
    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    duel.load_card("tests/fixtures/ExampleSpell.lua")
        .expect("ExampleSpell.lua should load");
    let spell = duel.add_to_hand(PLAYER_0, Card::new(12345678));

    duel.idle_command();
    duel.process();

    // Activate: the Spell moves to the field, then freezes for the target pick.
    duel.set_response(&[CMD_ACTIVATE, 0]);
    duel.process();
    assert_eq!(
        duel.zone_of(spell),
        Some(Zone::SpellTrapZone),
        "the activated Spell is on the field while it resolves"
    );

    // Resolve: pick the target, pass the response window(s) → Spell → graveyard.
    duel.set_response(&[0]);
    duel.process();
    pass_windows(&mut duel);
    assert_eq!(
        duel.zone_of(spell),
        Some(Zone::GY),
        "the resolved Spell is sent to the graveyard"
    );
}
