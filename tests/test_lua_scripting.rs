//! First light for the Lua card engine (PHASE-LUA M1).
//!
//! Loading a card runs its Lua source. The card builds itself with
//! `Card:new(id)` and registers each effect with `add_effect()`. After loading,
//! the duel should know about those effects.
//!
//! This pins the M1 contract only: load + register. No resolving, no cost, no
//! targeting yet — those are M2+.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Loading `Example.lua` runs it, and its single `add_effect()` registers one
/// effect on the duel.
#[test]
fn loading_a_card_registers_its_effect() {
    let mut duel = Duel::new();

    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    assert_eq!(
        duel.effect_count(),
        1,
        "the card's one add_effect() should register one effect"
    );
}

/// M2: running an effect's `resolve` stage drives the real duel. Example's
/// `resolve` is `e:destroy(e:targets())`, so once the effect has a target and we
/// resolve it, that card is sent to the graveyard.
///
/// No coroutines yet (that's M4): the target is injected directly, then we
/// resolve. This pins the `e:destroy` / `e:targets` verbs and the
/// "run the Lua stage, then apply what it did" flow.
#[test]
fn resolving_an_effect_destroys_its_target() {
    let mut duel = Duel::new();
    let monster = duel.add_card(Card);

    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    // Give effect 0 its target, then resolve it.
    duel.set_targets(vec![monster]);
    duel.resolve_effect(0).expect("resolve should run");

    assert_eq!(
        duel.zone_of(monster),
        Some(Zone::GY),
        "resolving e:destroy(e:targets()) should send the target to the GY"
    );
}

/// M3: activating an effect pays its cost. Example's cost is `e:pay_lp(500)`, so
/// paying the cost takes 500 from the ACTIVATING player — and nobody else.
#[test]
fn paying_an_effects_cost_deducts_lp_from_the_activating_player() {
    let mut duel = Duel::new();
    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    duel.pay_cost(0, PLAYER_0).expect("cost should be paid");

    assert_eq!(
        duel.life_points(PLAYER_0),
        7500,
        "e:pay_lp(500) should take 500 from the activating player"
    );
    assert_eq!(
        duel.life_points(PLAYER_1),
        8000,
        "the opponent's life points are untouched"
    );
}

/// M4 + M5: activating runs the `target` stage, which asks for a selection from
/// a real candidate set (the opponent's monsters). That YIELDS, freezing the
/// duel — the cost is paid, but nothing is destroyed yet. We pick a candidate
/// by index, resume, and the effect resolves and destroys it.
#[test]
fn activating_targets_and_destroys_an_opponent_monster() {
    let mut duel = Duel::new();
    let foe = duel.add_card(Card);
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    // Activate as player 0: cost paid, then the duel freezes for the pick.
    assert_eq!(
        duel.activate(0, PLAYER_0).expect("activate should run"),
        DuelStatus::Awaiting,
        "the target stage should yield and freeze the duel"
    );
    assert_eq!(
        duel.life_points(PLAYER_0),
        7500,
        "cost is paid before the pick"
    );
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "nothing is destroyed while we're still choosing"
    );

    // Pick candidate index 0 (the opponent's only monster) → resolve → destroy.
    duel.answer_selection(vec![0]);
    assert_eq!(duel.resume().expect("resume should run"), DuelStatus::End);
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "the chosen monster is destroyed after resuming"
    );
}

/// M5: `OPPONENT` is relative. The same card, activated by player 1, targets
/// player 0's monster.
#[test]
fn opponent_is_relative_to_the_activating_player() {
    let mut duel = Duel::new();
    let mine = duel.add_card(Card);
    duel.place(PLAYER_0, mine, Zone::MonsterZone);
    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    // Player 1 activates → their opponent is player 0 → only candidate is `mine`.
    assert_eq!(
        duel.activate(0, PLAYER_1).expect("activate should run"),
        DuelStatus::Awaiting
    );
    duel.answer_selection(vec![0]);
    duel.resume().expect("resume should run");

    assert_eq!(
        duel.zone_of(mine),
        Some(Zone::GY),
        "player 1's opponent is player 0"
    );
}

/// M5: no legal target → the activation is rejected up front. The candidate set
/// is empty, so the duel does not freeze and the cost is NOT paid.
#[test]
fn cannot_activate_with_no_legal_target() {
    let mut duel = Duel::new();
    // No monsters on the field — nothing to target.
    duel.load_card("cards/Example.lua")
        .expect("Example.lua should load");

    let status = duel.activate(0, PLAYER_0).expect("activate should run");

    assert_ne!(
        status,
        DuelStatus::Awaiting,
        "no legal target → must not freeze"
    );
    assert_eq!(
        duel.life_points(PLAYER_0),
        8000,
        "no legal target → the cost is not paid"
    );
}
