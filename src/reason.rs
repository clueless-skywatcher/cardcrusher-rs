//! Why a card left where it was — the *reason* for a movement/destruction.
//!
//! Mirrors EDOPro's `REASON_*` bitmask (`ocgcore/common.h`). A reason is a set
//! of flags so sources can combine (e.g. destruction by battle =
//! `REASON_DESTROY | REASON_BATTLE`). Once the event engine exists, triggers
//! like "when this card is destroyed by battle" test these bits on the card's
//! recorded reason — which is why destruction goes through `Duel::destroy` (it
//! records the reason) rather than a bare `send_to`.

pub type Reason = u32;

pub const REASON_DESTROY: Reason = 0x1;
pub const REASON_RELEASE: Reason = 0x2;
pub const REASON_BATTLE: Reason = 0x20;
pub const REASON_EFFECT: Reason = 0x40;
pub const REASON_COST: Reason = 0x80;
pub const REASON_RULE: Reason = 0x400;
