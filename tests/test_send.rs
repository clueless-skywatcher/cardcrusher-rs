//! Chain engine, rung 3: the `send(card, zone)` verb relocates a card to any zone.
//! Here it bounces an opponent's monster to the hand — and a `send` is NOT a
//! `destroy` (the card ends up in the Hand, not the GY).

use cardcrusher::card::Card;
use cardcrusher::duel::Duel;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{PLAYER_0, PLAYER_1};

#[test]
fn send_returns_a_monster_to_the_hand() {
    let mut duel = Duel::new();
    duel.load_card("tests/fixtures/Bounce.lua")
        .expect("Bounce.lua should load");

    let foe = duel.add_card(Card::new(0));
    duel.place(PLAYER_1, foe, Zone::MonsterZone);
    let spell = duel.make_card(90000007);
    let spell = duel.add_to_hand(PLAYER_0, spell);

    // Activate → pick the opponent's monster → resolve the chain.
    assert_eq!(
        duel.activate(spell, 0, PLAYER_0).expect("activate"),
        DuelStatus::Awaiting,
    );
    duel.answer_selection(vec![0]);
    duel.resume().expect("resume");
    duel.resolve_chain();

    assert_eq!(
        duel.zone_of(foe),
        Some(Zone::Hand),
        "the monster was sent to the hand (a bounce, not a destroy)",
    );
    // It's in the hand *pile*, not just the location map.
    assert_eq!(duel.hand_count(PLAYER_1), 1, "and it joined the hand pile");
}
