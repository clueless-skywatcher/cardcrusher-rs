use crate::{ids::CardId, reason::Reason};

pub const EVENT_DESTROYED: u32 = 1;
pub const EVENT_BATTLE_DESTROYED: u32 = 2;
/// Raised once a battle has fully resolved (mirrors `events.lua`). Subscriptions
/// keyed to it fire — e.g. Kuriboh removing its "no battle damage" modifier.
pub const EVENT_BATTLE_ENDED: u32 = 4;
/// The timing windows around damage calculation (mirror the prelude's
/// `events.lua`). A QUICK effect keyed to one of these (e.g. Kuriboh, at
/// PRE) may activate there. EDOPro opens equivalent windows turn-player-first
/// via `PointEvent` (processor.cpp, `BattleCommand` steps around 26/32).
pub const EVENT_PRE_DAMAGE_CALCULATION: u32 = 5;
pub const EVENT_POST_DAMAGE_CALCULATION: u32 = 6;

pub struct DuelEvent {
    pub code: u32,
    pub card: CardId,
    pub reason: Reason,
}
