#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Deck,
    Hand,
    MonsterZone,
    SpellTrapZone,
    GY,
    Banishment,
}

impl Zone {
    /// Map a `ZONE_*` code (from `prelude/zones.lua`, what the `send` verb passes)
    /// to a `Zone`. `None` for an unknown code.
    pub fn from_code(code: u32) -> Option<Zone> {
        Some(match code {
            0 => Zone::Deck,
            1 => Zone::Hand,
            2 => Zone::MonsterZone,
            3 => Zone::SpellTrapZone,
            4 => Zone::GY,
            5 => Zone::Banishment,
            _ => return None,
        })
    }
}
