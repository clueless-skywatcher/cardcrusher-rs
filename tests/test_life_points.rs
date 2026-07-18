//! Life points: costs and damage lower a player's LP.

use cardcrusher::duel::Duel;

/// Both players start at the standard 8000 life points.
#[test]
fn players_start_at_8000() {
    let duel = Duel::new();
    assert_eq!(duel.life_points(0), 8000);
    assert_eq!(duel.life_points(1), 8000);
}

/// Paying a cost deducts LP from that player only.
#[test]
fn paying_life_points_deducts_them() {
    let mut duel = Duel::new();

    duel.pay_lp(0, 500);

    assert_eq!(duel.life_points(0), 7500, "payer loses exactly 500");
    assert_eq!(duel.life_points(1), 8000, "the other player is untouched");
}

/// Damage lowers the damaged player's LP.
#[test]
fn damage_lowers_life_points() {
    let mut duel = Duel::new();

    duel.deal_damage(1, 2000);

    assert_eq!(duel.life_points(1), 6000);
}
