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

pub struct DuelEvent {
    pub code: u32,
    pub card: CardId,
    pub reason: Reason,
    pub details: BTreeMap<String, EventDetail>,
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
