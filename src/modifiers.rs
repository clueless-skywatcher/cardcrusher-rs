use crate::ids::CardId;

/// A modifier instance: a unique `id` (so one specific modifier can be removed —
/// EDOPro's `effect::id`), the `source` card that produced it (so all of a source's
/// modifiers can be removed together — EDOPro's `effect::owner`), and WHAT it
/// changes (`mod_type`).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Modifier {
    pub id: u32,
    pub source: CardId,
    pub mod_type: ModifierType,
}

/// The kinds of modifier a query gate folds. Each declares a fold `priority`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ModifierType {
    AtkChange(i32),
    DefChange(i32),
    SetAtk(i32),
    SetDef(i32),
    NoBattleDamage,
}

impl ModifierType {
    /// Fold priority — **lower applies first**, ties broken by insertion order.
    /// `SetAtk` (the base value) sits in an earlier band than `AtkChange` (which
    /// stacks on top), so `final = set value + Σ changes` regardless of the order
    /// they were added (EDOPro `get_attack`). `None` = unprioritized (not used yet).
    pub fn priority(&self) -> Option<i32> {
        match self {
            // A "set" is the base — an earlier band than the "+N" changes.
            ModifierType::SetAtk(_) | ModifierType::SetDef(_) => Some(0),
            ModifierType::AtkChange(_) | ModifierType::DefChange(_) => Some(1),
            // Not part of the ATK/DEF value fold — a boolean gate read at damage
            // time, so it has no fold priority.
            ModifierType::NoBattleDamage => None,
        }
    }

    /// Build a `ModifierType` from the `MOD_*` code Lua passes (see
    /// `prelude/modifiers.lua`) plus its value (ignored by value-less kinds).
    /// `None` for an unknown code.
    pub fn from_code(code: u32, value: i32) -> Option<Self> {
        match code {
            0 => Some(ModifierType::AtkChange(value)),
            1 => Some(ModifierType::DefChange(value)),
            2 => Some(ModifierType::SetAtk(value)),
            3 => Some(ModifierType::SetDef(value)),
            4 => Some(ModifierType::NoBattleDamage),
            _ => None,
        }
    }
}
