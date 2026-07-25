//! Once-per-turn Normal Summon: a player may Normal Summon only once per turn,
//! and the limit resets at the start of each new turn.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::{CMD_NEXT_PHASE, CMD_SUMMON, PLAYER_0, PLAYER_1};

/// A second Normal Summon in the same turn is rejected.
#[test]
fn only_one_normal_summon_per_turn() {
    let mut duel = Duel::new();

    // Player 0 has two monsters in hand.
    duel.add_to_hand(PLAYER_0, Card::new(0));
    duel.add_to_hand(PLAYER_0, Card::new(0));

    duel.idle_command();
    assert_eq!(duel.process(), DuelStatus::Awaiting);

    // First Normal Summon (hand slot 0) lands.
    duel.set_response(&[CMD_SUMMON, 0]);
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(duel.monster_zone(PLAYER_0).len(), 1, "first summon lands");

    // Second Normal Summon this turn (the other card, now at slot 0) is blocked.
    duel.set_response(&[CMD_SUMMON, 0]);
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(
        duel.monster_zone(PLAYER_0).len(),
        1,
        "a second Normal Summon in one turn is rejected"
    );
    assert_eq!(
        duel.hand_count(PLAYER_0),
        1,
        "the blocked card stays in hand"
    );
}

/// The limit resets each turn: over three turns (p0, p1, p0), player 0 lands one
/// Normal Summon per turn — two total. Without a reset it would be stuck at one.
#[test]
fn the_normal_summon_limit_resets_each_turn() {
    let mut duel = Duel::new();
    duel.set_max_turns(3); // p0, p1, p0

    // Decks so nobody decks out; monsters in p0's hand to summon each turn.
    for _ in 0..10 {
        duel.add_to_deck(PLAYER_0, Card::new(0));
        duel.add_to_deck(PLAYER_1, Card::new(0));
    }
    for _ in 0..3 {
        duel.add_to_hand(PLAYER_0, Card::new(0));
    }

    duel.start();

    // Drive: at each menu, try one summon (slot 0), then advance the phase. The
    // toggle guarantees forward progress (every other answer is "next phase").
    let mut tried_summon = false;
    loop {
        match duel.process() {
            DuelStatus::End => break,
            DuelStatus::Awaiting => {
                if tried_summon {
                    duel.set_response(&[CMD_NEXT_PHASE]);
                    tried_summon = false;
                } else {
                    duel.set_response(&[CMD_SUMMON, 0]);
                    tried_summon = true;
                }
            }
            DuelStatus::Continue => unreachable!("process runs until End or Awaiting"),
        }
    }

    assert_eq!(
        duel.monster_zone(PLAYER_0).len(),
        2,
        "one Normal Summon per turn × two of player 0's turns"
    );
}
