//! What a card actually *does*.
//!
//! **Milestone 1:** empty room.
//!
//! **The big idea:** a card's text (e.g. "destroy 1 monster your opponent
//! controls") gets turned into a fixed data shape the engine can run — the
//! *effect IR*. Rough sketch of what that shape will hold later:
//!
//! ```text
//! type        // activated? continuous? a trigger?
//! range       // where does it work from — hand? field?
//! condition   // when is it allowed to happen?
//! cost        // what must I pay first? (e.g. 500 LP)
//! target      // what does it point at? (1 opponent monster)
//! operation   // what does it do? (destroy)
//! count_limit // "once per turn", etc.
//! ```
//!
//! **Why this matters:** the IR is the engine's *permanent* contract. The card
//! language on top (Lua now, maybe a custom one later) is just a friendly way to
//! emit this IR — so we can swap the language without touching the engine.
#[derive(Debug, Default)]
pub struct Effect;
