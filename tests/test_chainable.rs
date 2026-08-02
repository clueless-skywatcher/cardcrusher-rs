//! Chain engine, rung 4: `chainable_effects` — which effects a player may activate
//! **in response** to the current chain. The gate is spell speed: an effect may
//! respond only if its speed is `>= 2` (SS1 never responds) **and** `>= the top
//! link's speed`. These pin that gate.

use cardcrusher::card::Card;
use cardcrusher::chain::ChainLink;
use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// A top chain link whose effect is an SS1 normal spell (Nuke). Built by hand so
/// the test needn't reach into the private `chain`. `effect_seq = 0` is fine: for
/// a spell activation the speed comes from the *card's* type/subtype, and the
/// effect is only read for its (ACTIVATE) kind.
fn ss1_top_link(duel: &mut Duel) -> ChainLink {
    let card = duel.make_card(90000006); // Nuke → normal spell → speed 1
    let card = duel.add_card(card);
    ChainLink {
        effect_seq: 0,
        card,
        activator: PLAYER_0,
        targets: vec![],
        event: Default::default(),
    }
}

/// Against an SS1 top link, a quick-play spell (SS2) may respond but a normal
/// spell (SS1) may not.
#[test]
fn only_fast_effects_may_respond_to_a_slow_top_link() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Nuke.lua")
        .expect("Nuke.lua should load");
    duel.load_card("tests/fixtures/QuickNuke.lua")
        .expect("QuickNuke.lua should load");

    let top = ss1_top_link(&mut duel);

    // Quick Nuke's condition needs an opponent monster to wipe — P1's opponent is
    // P0, so give P0 a monster.
    let target = duel.add_card(Card::new(0));
    duel.place(PLAYER_0, target, Zone::MonsterZone);

    // P1 holds a quick-play (SS2) and a normal spell (SS1).
    let quick = duel.make_card(90000008);
    let quick = duel.add_to_hand(PLAYER_1, quick);
    let slow = duel.make_card(90000006);
    duel.add_to_hand(PLAYER_1, slow);

    let out = duel.chainable_effects(PLAYER_1, &top);
    assert_eq!(
        out.len(),
        1,
        "only the quick-play may respond to an SS1 link"
    );
    assert_eq!(out[0].0, quick, "and it's the quick-play spell");
}

/// A hand of only Spell-Speed-1 spells can respond with nothing.
#[test]
fn a_slow_only_hand_can_respond_with_nothing() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Nuke.lua")
        .expect("Nuke.lua should load");

    let top = ss1_top_link(&mut duel);

    let slow = duel.make_card(90000006);
    duel.add_to_hand(PLAYER_1, slow);

    assert!(
        duel.chainable_effects(PLAYER_1, &top).is_empty(),
        "a Spell-Speed-1 spell can never respond to a chain",
    );
}
