use crate::event::EventSnapshot;
use crate::ids::CardId;

pub struct ChainLink {
    pub effect_seq: usize,
    pub card: CardId,
    pub activator: usize,
    pub targets: Vec<CardId>,
    /// The event that fired this link (for a trigger), so its details are readable
    /// at resolution. `Default` (code 0) for a non-trigger activation.
    pub event: EventSnapshot,
}
