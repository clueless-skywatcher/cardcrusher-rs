//! Chain engine, C5: SEGOC (Simultaneous Effects Go On Chain). When several
//! triggers fire at once, they're placed on the chain in a defined order —
//! EDOPro-confirmed: turn-player's forced → opponent's forced → turn-player's
//! optional → opponent's optional (`Processors::PointEvent`). We handle the
//! mandatory (forced) buckets: **turn player's trigger is placed first**, so
//! under LIFO the opponent's resolves first.
//!
//! We observe the *placement order* directly via `chain_activators()` (the
//! activator of each link, chain-order) — resolution order for no-target effects
//! isn't observable (two board-wipes both wipe regardless of order), and targeting
//! triggers are deferred.
//!
//! NEW engine surface: `Duel::chain_activators(&self) -> Vec<usize>`.

use cardcrusher::duel::Duel;
use cardcrusher::reason::REASON_EFFECT;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Two mandatory-trigger monsters — one the turn player's (P0), one the
/// opponent's (P1) — destroyed **opponent-first**, so event order is the REVERSE
/// of SEGOC order. SEGOC must still place the turn player's trigger on link 1.
#[test]
fn segoc_places_turn_players_trigger_before_the_opponents() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Retaliator.lua")
        .expect("Retaliator.lua should load");

    // No turn started → the turn player defaults to P0.
    let mine = duel.make_card(90000003);
    let mine = duel.add_card(mine);
    duel.place(PLAYER_0, mine, Zone::MonsterZone);
    let theirs = duel.make_card(90000003);
    let theirs = duel.add_card(theirs);
    duel.place(PLAYER_1, theirs, Zone::MonsterZone);

    // Destroy OPPONENT-FIRST: raises P1's trigger event before P0's, so event
    // order ([P1, P0]) is the reverse of the SEGOC order we expect.
    duel.destroy(theirs, REASON_EFFECT);
    duel.destroy(mine, REASON_EFFECT);
    duel.process_events();

    assert_eq!(
        duel.chain_activators(),
        vec![PLAYER_0, PLAYER_1],
        "SEGOC: the turn player's trigger is placed first (link 1), regardless of \
         the order the events were raised",
    );
}
