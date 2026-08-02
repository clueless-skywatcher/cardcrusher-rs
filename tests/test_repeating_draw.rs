//! A rule that fires **more than once** and then expires — the Maxx "C" shape.
//!
//! `DrawEachBattle` is a tracked mini-Maxx "C": discard it from hand, and for the
//! rest of the turn every battle that ends draws you 1 card. Where Kuriboh's
//! `queue(..., {1, ONCE}, ...)` fires exactly once and dies, this one repeats until
//! its expiry event arrives.
//!
//! ASSUMED API (these tests pin it — implement to match):
//! - `e:draw(who, n)` verb — draw `n` cards for `who` (YOU/OPPONENT, relative to the
//!   activator). Cards land in that player's hand; the deck shrinks by the same.
//! - `e:apply_event_until(on_event, until_event, fn)` verb — run `fn` every time
//!   `on_event` fires, until `until_event` fires. No count; the expiry ends it.
//!   `fn` receives `(ev, until_ev)`, exactly one of which is non-nil: `ev` on each
//!   repeat, `until_ev` on the single final call as the rule is removed.
//! - The rule captures its **owner** at registration, so `YOU` keeps meaning the
//!   activating player long after the effect finished resolving.
//! - `EVENT_TURN_ENDED` is raised once the End Phase is fully done, and everything
//!   listening to an event fires before anything expiring on it is removed.

use cardcrusher::card::{Card, CardData};
use cardcrusher::duel::Duel;
use cardcrusher::position::Position;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ATTACK, CMD_NEXT_PHASE, MSG_SELECT_ATTACK_TARGET, MSG_SELECT_BATTLECMD};
use cardcrusher::{PLAYER_0, PLAYER_1};

const DRAW_EACH_BATTLE: u32 = 90000019;

/// Put `n` blank cards in `player`'s deck so there's something to draw.
fn stock_deck(duel: &mut Duel, player: usize, n: usize) {
    for _ in 0..n {
        duel.add_to_deck(player, Card::new(0));
    }
}

/// Give `player` a face-up attack-position monster with `atk`.
fn give_attacker(duel: &mut Duel, player: usize, atk: i32) {
    let m = duel.add_card(Card::with_data(
        1,
        CardData {
            atk,
            ..Default::default()
        },
    ));
    duel.place(player, m, Zone::MonsterZone);
    duel.change_position(m, Position::FaceUpAttack);
}

/// The whole point: it fires on EVERY battle, not just the first.
///
/// P1 discards the rule, then P0 swings twice with two different monsters. Two
/// battles end → two draws. A `queue(..., {1, ONCE}, ...)`-style one-shot would
/// draw exactly once and this fails at 1.
#[test]
fn a_repeating_rule_draws_once_per_battle() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/DrawEachBattle.lua")
        .expect("DrawEachBattle.lua should load");

    stock_deck(&mut duel, PLAYER_1, 10);

    // P1 holds the rule and activates it (turn player is P0, so `current_player()`
    // reads as OPPONENT and the condition passes).
    let rule = duel.make_card(DRAW_EACH_BATTLE);
    let rule = duel.add_to_hand(PLAYER_1, rule);
    let _ = duel.activate(rule, 0, PLAYER_1);
    duel.resolve_chain();

    assert_eq!(
        duel.zone_of(rule),
        Some(Zone::GY),
        "sent from hand to GY as the activation cost"
    );

    // Baseline AFTER the discard cost, so we're only measuring draws.
    let deck_before = duel.deck_count(PLAYER_1);
    let hand_before = duel.hand_count(PLAYER_1);

    // P0 controls two monsters; P1 controls none, so both swings are direct attacks
    // (no target-selection step to drive).
    give_attacker(&mut duel, PLAYER_0, 1000);
    give_attacker(&mut duel, PLAYER_0, 1000);

    // Battle 1.
    duel.battle_command();
    duel.process();
    duel.set_response(&[CMD_ATTACK, 0]);
    duel.process();

    assert_eq!(
        duel.deck_count(PLAYER_1),
        deck_before - 1,
        "first battle ended → drew 1"
    );

    // Battle 2 — the spent attacker has dropped out, so index 0 is the other one.
    duel.set_response(&[CMD_ATTACK, 0]);
    duel.process();

    assert_eq!(
        duel.deck_count(PLAYER_1),
        deck_before - 2,
        "second battle ended → drew again (the rule REPEATS, it is not a one-shot)"
    );
    assert_eq!(
        duel.hand_count(PLAYER_1),
        hand_before + 2,
        "both drawn cards landed in the activating player's hand"
    );
}

/// It stops at the end of the turn it was used on.
///
/// Three turns (p0, p1, p0). The rule is installed before turn 1, draws on turn 1's
/// battle, then `EVENT_TURN_ENDED` removes it — so turn 3's battle draws nothing.
/// Without an expiry, turn 3 draws too and the final assert fails.
#[test]
fn the_repeating_rule_expires_when_the_turn_ends() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/DrawEachBattle.lua")
        .expect("DrawEachBattle.lua should load");
    duel.set_max_turns(3); // p0, p1, p0

    stock_deck(&mut duel, PLAYER_0, 20);
    stock_deck(&mut duel, PLAYER_1, 20);

    // P0's lone monster attacks directly on each of their turns.
    give_attacker(&mut duel, PLAYER_0, 1000);

    // P1 installs the rule during P0's (first) turn.
    let rule = duel.make_card(DRAW_EACH_BATTLE);
    let rule = duel.add_to_hand(PLAYER_1, rule);
    let _ = duel.activate(rule, 0, PLAYER_1);
    duel.resolve_chain();

    duel.start();

    // Drive: one attack per Battle Phase, skip every other menu. Measure P1's deck
    // across each battle in isolation (snapshot when the attack is declared, read it
    // back when the menu reopens) so P1's own turn-2 draw can't pollute the numbers.
    let mut drawn_per_turn: Vec<(usize, usize)> = Vec::new();
    let mut pending: Option<(usize, usize)> = None; // (turn, deck before the battle)
    loop {
        match duel.process() {
            DuelStatus::End => break,
            DuelStatus::Awaiting => match duel.messages().last().copied() {
                Some(MSG_SELECT_BATTLECMD) => {
                    if let Some((turn, deck_before)) = pending.take() {
                        drawn_per_turn.push((turn, deck_before - duel.deck_count(PLAYER_1)));
                        duel.set_response(&[CMD_NEXT_PHASE]);
                    } else {
                        pending = Some((duel.turn_history().len(), duel.deck_count(PLAYER_1)));
                        duel.set_response(&[CMD_ATTACK, 0]);
                    }
                }
                Some(MSG_SELECT_ATTACK_TARGET) => duel.set_response(&[0]),
                _ => duel.set_response(&[CMD_NEXT_PHASE]), // Main-Phase menus
            },
            DuelStatus::Continue => unreachable!("process runs until End or Awaiting"),
        }
    }

    let drawn_on = |turn: usize| {
        drawn_per_turn
            .iter()
            .find(|(t, _)| *t == turn)
            .unwrap_or_else(|| panic!("turn {turn} should have had a battle"))
            .1
    };

    assert_eq!(
        drawn_on(1),
        1,
        "the rule is live on turn 1 → that battle draws 1"
    );
    assert_eq!(
        drawn_on(3),
        0,
        "turn 1 ended → the rule expired → turn 3's battle draws nothing",
    );
}
