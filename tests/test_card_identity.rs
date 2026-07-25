//! Card identity + card↔effect linkage.
//!
//! A card's `.lua` script is the single source of truth: `Card:new(code)`
//! self-registers the card with the engine, and each `add_effect()` is linked to
//! that card's code. So the engine can answer "what effects does card <code>
//! have?" — the basis for activating an effect off a real card instead of a flat
//! global index.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Step 1: loading `Example.lua` (`Example = Card:new(12345678)` with one
/// `add_effect()`) registers the card definition and links its one effect to
/// that code.
#[test]
fn a_loaded_card_links_its_effect_to_its_code() {
    let mut duel = Duel::new();

    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    assert_eq!(
        duel.code_effects(12345678).len(),
        1,
        "the card defined for code 12345678 should own its one effect"
    );
    assert_eq!(
        duel.code_effects(99999999).len(),
        0,
        "an unknown code owns no effects"
    );
}

/// Step 2: a card instance carries a code, and finds its effects through it —
/// the link from "a card on the board" to "what it does".
#[test]
fn a_card_instance_finds_its_effects() {
    let mut duel = Duel::new();
    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    let card = duel.add_card(Card::new(12345678));

    assert_eq!(
        duel.effects_of(card).len(),
        1,
        "the instance's code links it to the loaded definition's effect"
    );
}

/// Step 3: activation is card-driven. You activate a specific card instance's
/// effect by slot — not a flat global index.
#[test]
fn activating_a_cards_effect_runs_it() {
    let mut duel = Duel::new();
    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    let spell = duel.add_card(Card::new(12345678));

    // Activate THIS card's effect (slot 0), as player 0.
    assert_eq!(
        duel.activate(spell, 0, PLAYER_0)
            .expect("activate should run"),
        DuelStatus::Awaiting
    );
    duel.answer_selection(vec![0]);
    duel.resume().expect("resume should run");
    assert_eq!(duel.zone_of(foe), Some(Zone::GY));
}

/// Step 3: activation is gated on `condition`. CantActivate's condition returns
/// false, so activating it is rejected — it does not freeze and pays no cost.
#[test]
fn a_false_condition_blocks_activation() {
    let mut duel = Duel::new();
    duel.load_card("cards/CantActivate.lua")
        .expect("CantActivate.lua should load");

    let card = duel.add_card(Card::new(11111111));
    let status = duel
        .activate(card, 0, PLAYER_0)
        .expect("activate should run");

    assert_ne!(
        status,
        DuelStatus::Awaiting,
        "a false condition must not activate or freeze"
    );
    assert_eq!(
        duel.life_points(PLAYER_0),
        8000,
        "a rejected activation pays no cost"
    );
}
