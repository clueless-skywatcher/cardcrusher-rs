use std::collections::BTreeMap;

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
pub const EVENT_SPECIAL_SUMMON: u32 = 7;
/// The End Phase has begun — for things that happen *during* it.
pub const EVENT_END_PHASE_STARTED: u32 = 8;
/// The End Phase is fully done. Where "this turn" rules expire (see
/// `Subscription::until`). Everything listening to an event fires before anything
/// expiring on it is removed — EDOPro does the same, `PointEvent` (processor.cpp:557)
/// before `reset_phase` (`:563`).
pub const EVENT_TURN_ENDED: u32 = 9;

pub struct DuelEvent {
    pub code: u32,
    /// The card the event is "about" — `None` for a *global* event that isn't tied
    /// to one (a battle ending, a turn ending). Triggers only match when it's
    /// `Some`; subscriptions fire either way.
    pub card: Option<CardId>,
    pub reason: Reason,
    pub details: BTreeMap<String, EventDetail>,
}

impl DuelEvent {
    /// An event with no card and no details — "a battle ended", "the turn ended".
    pub fn global(code: u32) -> Self {
        DuelEvent {
            code,
            card: None,
            reason: 0,
            details: BTreeMap::new(),
        }
    }
}

/// A value carried in an event's detail bag, queried from Lua by
/// `e:get_event_detail(code, key)`.
#[derive(Clone, Debug)]
pub enum EventDetail {
    Card(CardId),
    Cards(Vec<CardId>),
    Int(i32),
    Bool(bool),
}

/// The frozen `(code, details)` of the event that fired a trigger, carried on its
/// chain link so the details are still readable when the effect resolves later.
/// `Default` (code 0) means "no event" — e.g. a plain Spell/Trap activation.
#[derive(Clone, Default, Debug)]
pub struct EventSnapshot {
    pub code: u32,
    pub details: BTreeMap<String, EventDetail>,
}
