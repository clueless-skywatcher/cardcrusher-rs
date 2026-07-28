use crate::{ids::CardId, reason::Reason};

pub const EVENT_DESTROYED: u32 = 1;
pub const EVENT_BATTLE_DESTROYED: u32 = 2;

pub struct DuelEvent {
    pub code: u32,
    pub card: CardId,
    pub reason: Reason,
}
