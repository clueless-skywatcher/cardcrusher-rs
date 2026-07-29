//! The chain engine, milestone C0: activation is split from resolution.
//!
//! Today `activate()` resolves an effect on the spot. A real duel puts the
//! activation on a **chain** first, and resolves it as a separate step (so C1/C2
//! can slot an opponent response window + LIFO into the gap). C0 pins that split:
//!
//! - activating an effect leaves it **on the chain, unresolved**;
//! - `resolve_chain()` walks the chain and applies each link, emptying it.
//!
//! NEW engine surface this test assumes (to implement for C0):
//! - `Duel::chain_len(&self) -> usize` — how many links are on the chain.
//! - `Duel::resolve_chain(&mut self)` — resolve every link LIFO, then clear.
//! - `activate()` no longer resolves inline; it pushes a `ChainLink` and stops.
//!
//! Mirrors EDOPro: `AddChain` pushes the link (`processor.cpp:3626`), `SolveChain`
//! resolves it later (`processor.cpp:4096`).

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::ids::CardId;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Nuke (no-target destroy-all) in P0's hand, one foe on P1's field to wipe.
/// Returns (nuke id, foe id).
fn setup_nuke() -> (Duel, CardId, CardId) {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Nuke.lua")
        .expect("Nuke.lua should load");

    let nuke = duel.make_card(90000006);
    let nuke = duel.add_to_hand(PLAYER_0, nuke);

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    (duel, nuke, foe)
}

/// Activating puts the effect on the chain but does NOT resolve it; resolving the
/// chain then applies the effect and empties the chain.
#[test]
fn activation_goes_on_the_chain_then_resolves() {
    let (mut duel, nuke, foe) = setup_nuke();

    // Declare the activation → it lands on the chain, unresolved.
    duel.activate(nuke, 0, PLAYER_0).expect("activate Nuke");
    assert_eq!(duel.chain_length(), 1, "the activation is on the chain");
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::MonsterZone),
        "nothing has resolved yet",
    );

    // Resolve the chain → the effect happens and the chain empties.
    duel.resolve_chain();
    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::GY),
        "resolving the chain wiped the opponent's board",
    );
    assert_eq!(duel.chain_length(), 0, "the chain is empty after resolving");
}
