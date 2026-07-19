//! The card DSL: scripts describe effects, and stored `resolve` closures run
//! later and drive the real duel.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::zone::Zone;

/// Loading a card runs its entry function, registering exactly one activate
/// effect. Registering does nothing else yet.
#[test]
fn loading_a_card_registers_its_effect() {
    let mut duel = Duel::new();

    duel.load_card("cards/Example.rhai")
        .expect("Example.rhai should load");

    assert_eq!(duel.effect_count(), 1);
}

/// The payoff: a `resolve` closure defined when the card loaded runs LATER and
/// destroys the targeted card — sending it to the graveyard.
#[test]
fn resolving_an_effect_destroys_the_targeted_card() {
    let mut duel = Duel::new();
    let monster = duel.add_card(Card);

    duel.load_card("cards/Example.rhai")
        .expect("Example.rhai should load");

    duel.answer_target(vec![monster]);
    duel.resolve(0);

    assert_eq!(
        duel.zone_of(monster),
        Some(Zone::GY),
        "destroy should send the card to the graveyard"
    );
}
