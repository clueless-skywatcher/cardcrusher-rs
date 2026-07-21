//! Activating an effect pays its cost.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::{CMD_ACTIVATE, PLAYER_0};

/// The Example card costs `PayLP(500)`. Activating it deducts 500 from the
/// activating player's life points (paid at activation, before it resolves).
#[test]
fn activating_an_effect_pays_its_lp_cost() {
    let mut duel = Duel::new();
    duel.add_card(Card); // a target to keep the effect happy
    duel.load_card("cards/Example.rhai")
        .expect("Example.rhai should load");

    duel.idle_command();
    duel.process(); // menu → Awaiting

    // Activate effect 0.
    duel.set_response(&[CMD_ACTIVATE, 0]);
    duel.process(); // activate: pay cost, then freeze for a target

    assert_eq!(
        duel.life_points(PLAYER_0),
        7500,
        "activating should have paid the 500 LP cost"
    );
}
