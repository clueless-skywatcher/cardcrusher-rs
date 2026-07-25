//! A monster's **battle position** while it sits in a Monster Zone.
//!
//! Two axes: **attack vs defense**, and **face-up vs face-down**. Only these four
//! combos exist, so it's a plain enum (determinism-friendly, exhaustive) rather
//! than EDOPro's `POS_*` bitflags (`ocgcore/common.h`) — same meaning.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Normal Summon: face-up, ready to attack.
    #[default]
    FaceUpAttack,
    /// Face-down but attacking — rare, only via specific effects.
    FaceDownAttack,
    /// Face-up, defending.
    FaceUpDefense,
    /// Set: hidden, defending.
    FaceDownDefense,
}

impl Position {
    /// In an attack position (its ATK is what the Battle Phase uses).
    pub fn is_attack(self) -> bool {
        matches!(self, Position::FaceUpAttack | Position::FaceDownAttack)
    }

    /// In a defense position (its DEF is what the Battle Phase uses).
    pub fn is_defense(self) -> bool {
        !self.is_attack()
    }

    /// Visible to both players.
    pub fn is_face_up(self) -> bool {
        matches!(self, Position::FaceUpAttack | Position::FaceUpDefense)
    }

    /// Hidden (set) — its identity isn't public.
    pub fn is_face_down(self) -> bool {
        !self.is_face_up()
    }
}
