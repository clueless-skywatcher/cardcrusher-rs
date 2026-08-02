//! Kuriboh rung 2: the **discard-self cost**. Activating `SelfDiscard` should pay
//! its cost by sending that very card (hand → GY). Depends on rung 1 (`self_card`),
//! since "discard self" needs to know which instance is activating.
//!
//! NEW surface assumed: `CostType::Discard(CardId)` + a Lua verb `e:discard_self()`
//! that records self_card as a discard cost; the duel's `apply_cost` moves it to GY.

use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;
use cardcrusher::PLAYER_0;

const SELF_DISCARD: u32 = 90000012;

#[test]
fn discard_self_pays_the_cost_from_the_hand() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/SelfDiscard.lua")
        .expect("SelfDiscard.lua should load");

    let card = duel.make_card(SELF_DISCARD);
    let card = duel.add_to_hand(PLAYER_0, card);

    let _ = duel.activate(card, 0, PLAYER_0);

    assert_eq!(
        duel.zone_of(card),
        Some(Zone::GY),
        "the activation cost discarded this card (hand → GY)",
    );
    assert_eq!(
        duel.chain_length(),
        1,
        "and the effect itself still went on the chain",
    );
}
