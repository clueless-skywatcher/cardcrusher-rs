#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Deck,
    Hand,
    MonsterZone,
    SpellTrapZone,
    GY,
    Banishment,
}
