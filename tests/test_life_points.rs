//! Life points: costs and damage lower a player's LP.

use cardcrusher::duel::Duel;
use cardcrusher::{PLAYER_0, PLAYER_1};

/// Both players start at the standard 8000 life points.
#[test]
fn players_start_at_8000() {
    let duel = Duel::new();
    assert_eq!(duel.life_points(PLAYER_0), 8000);
    assert_eq!(duel.life_points(PLAYER_1), 8000);
}

/// Paying a cost deducts LP from that player only.
#[test]
fn paying_life_points_deducts_them() {
    let mut duel = Duel::new();

    duel.pay_lp(PLAYER_0, 500);

    assert_eq!(duel.life_points(PLAYER_0), 7500, "payer loses exactly 500");
    assert_eq!(
        duel.life_points(PLAYER_1),
        8000,
        "the other player is untouched"
    );
}

/// Damage lowers the damaged player's LP.
#[test]
fn damage_lowers_life_points() {
    let mut duel = Duel::new();

    duel.deal_damage(PLAYER_1, 2000);

    assert_eq!(duel.life_points(PLAYER_1), 6000);
}
