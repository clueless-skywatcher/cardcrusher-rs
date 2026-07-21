//! Real targeting: an effect can only affect a card from its candidate set
//! (here, "a monster the opponent controls"), not any card the host names.

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{CMD_ACTIVATE, PLAYER_0, PLAYER_1};

/// A legal target — an opponent's monster — is destroyed.
#[test]
fn an_effect_destroys_a_legal_target() {
    let mut duel = Duel::new();
    let foe = duel.add_card(Card);
    duel.place(PLAYER_1, foe, Zone::MonsterZone); // opponent's monster
    duel.load_card("cards/Example.rhai")
        .expect("Example.rhai should load");

    duel.idle_command();
    duel.process(); // menu
    duel.set_response(&[CMD_ACTIVATE, 0]);
    assert_eq!(duel.process(), DuelStatus::Awaiting, "freezes to pick a target");

    duel.answer_target(vec![foe]);
    duel.process();

    assert_eq!(duel.zone_of(foe), Some(Zone::GY));
}

/// An illegal target — a card YOU control — is rejected: the effect leaves it
/// alone even if the host names it, because it isn't in the candidate set.
#[test]
fn a_card_you_control_is_not_a_legal_target() {
    let mut duel = Duel::new();
    let foe = duel.add_card(Card);
    duel.place(PLAYER_1, foe, Zone::MonsterZone); // a real candidate exists
    let mine = duel.add_card(Card);
    duel.place(PLAYER_0, mine, Zone::MonsterZone); // your own monster

    duel.load_card("cards/Example.rhai")
        .expect("Example.rhai should load");

    duel.idle_command();
    duel.process();
    duel.set_response(&[CMD_ACTIVATE, 0]);
    duel.process();

    // Try to target your own monster — not a legal target.
    duel.answer_target(vec![mine]);
    duel.process();

    assert_eq!(
        duel.zone_of(mine),
        Some(Zone::MonsterZone),
        "your own monster isn't a legal target, so it survives"
    );
}
