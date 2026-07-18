//! Coat-check tickets: how objects refer to each other.
//!
//! **The problem we're dodging:** without this, a card would hold a raw *pointer*
//! to (say) the card it's equipped to. If that other card gets destroyed, the
//! pointer still "works" — it now points at garbage. Read it → crash or worse.
//!
//! **Our fix:** nobody holds pointers. Every object lives in a big arena (a
//! [`slotmap::SlotMap`], think "coat-check room") and you refer to it by a tiny
//! copyable *ticket*: a `CardId`, `EffectId`, or `GroupId`.
//!
//! **The clever part — generations.** These tickets carry a generation number.
//! Free slot #5 and reuse it, and the new occupant becomes "#5, gen 2". Your old
//! "#5, gen 1" ticket no longer matches, so the lookup safely returns `None`
//! instead of handing you the wrong card.
//!
//! ```text
//! let a = cards.insert(card);   // a = "slot 5, gen 1"
//! cards.remove(a);              // slot 5 is now empty
//! let b = cards.insert(card);   // b = "slot 5, gen 2"  (reused the slot!)
//!
//! cards.get(a)  // => None   ✅ old ticket is dead, no crash
//! cards.get(b)  // => Some(..) ✅ new ticket works
//! ```
//!
//! That "old ticket → None" behaviour is a use-after-free guard — except we get
//! it for free and it can never crash. Proven by a test in `test_arena`.

use slotmap::new_key_type;

// `new_key_type!` stamps out a distinct ticket type for each arena. They're all
// tiny `Copy` structs under the hood — cheap to pass around by value.
new_key_type! {
    /// Ticket for one [`crate::card::Card`] in the duel's card arena.
    pub struct CardId;

    /// Ticket for one [`crate::effect::Effect`] in the duel's effect arena.
    pub struct EffectId;

    /// Ticket for one [`crate::group::Group`] in the duel's group arena.
    pub struct GroupId;
}
