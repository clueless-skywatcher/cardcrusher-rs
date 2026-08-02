//! Kuriboh rung 1: `self_card` tracking. An effect must know WHICH card instance
//! is activating it, so verbs like `in_hand()` / `discard_self()` can act on that
//! card. Pinned via `HandOnly`, whose only condition is `in_hand(YOU)`.
//!
//! NEW surface assumed: `EffectContext.self_card` (set on activation) + a Lua verb
//! `e:in_hand(who)` — is self_card in `who`'s hand (YOU/OPPONENT, relative to the
//! activator)?

use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::PLAYER_0;

const HAND_ONLY: u32 = 90000010;

/// `in_hand(YOU)` must discriminate on WHERE the card is — same card instance:
/// activatable in the hand, rejected once it's on the field. One test so a broken
/// verb (which just makes every activation fail) can't sneak past as a half-pass.
#[test]
fn in_hand_gates_activation_on_the_self_card_location() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/HandOnly.lua")
        .expect("HandOnly.lua should load");

    let card = duel.make_card(HAND_ONLY);
    let card = duel.add_to_hand(PLAYER_0, card);

    // In the hand → in_hand(YOU) true → the activation lands on the chain.
    let _ = duel.activate(card, 0, PLAYER_0);
    assert_eq!(
        duel.chain_length(),
        1,
        "in the hand, in_hand(YOU) is true → it activates",
    );

    // Move the SAME card to the field, then try again → in_hand(YOU) false → rejected.
    duel.resolve_chain(); // clear the chain from the first activation
    duel.place(PLAYER_0, card, Zone::SpellTrapZone);
    let _ = duel.activate(card, 0, PLAYER_0);
    assert_eq!(
        duel.chain_length(),
        0,
        "once on the field, in_hand(YOU) is false → it's rejected",
    );
}
