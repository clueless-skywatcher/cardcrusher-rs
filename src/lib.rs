//! # cardcrusher
//!
//! **What this is:** a Yu-Gi-Oh! rules *referee*, written from scratch in Rust.
//!
//! **The one big idea:** the engine knows the *rules* but knows *nothing* about
//! any specific card. Each card is a tiny script that says "hey referee, do X".
//! That's how one engine can support thousands of cards without changing.
//!
//! ---
//!
//! ## 🎲 House rules: DETERMINISM (read this, it shapes everything)
//!
//! **The promise:** same seed + same player choices → the *exact same game*,
//! byte for byte, forever. That's what makes replays and online play possible.
//!
//! **Why you can't add it later:** if two players' computers ever disagree by
//! even one bit, the game desyncs. So determinism is a rule we bake into every
//! file from line one. Five rules, no exceptions:
//!
//! 1. **One dice bag.** All randomness comes from ONE seeded PRNG
//!    ([`rand_xoshiro::Xoshiro256StarStar`]). Never `rand::random()`, never seed
//!    from the clock. Same seed → same shuffles, forever.
//! 2. **No decimals in game logic.** Integers only. Floats round differently on
//!    different machines → desync. (Life points, ATK, etc. are all whole numbers
//!    anyway.)
//! 3. **Sort, THEN loop.** If loop order could change the outcome, sort first by
//!    a stable key (usually an id). Never leave order to chance.
//! 4. **IDs, not pointers.** Objects find each other by id number, not memory
//!    address. Addresses are random per run; ids are not. (See [`ids`].)
//! 5. **Ordered maps only.** Use [`std::collections::BTreeMap`] /
//!    [`BTreeSet`](std::collections::BTreeSet) (or a sorted `Vec`) anywhere the
//!    game state cares. **Never** `HashMap`/`HashSet` — their order can shuffle
//!    between runs and silently change the game.
//!
//! > 🚨 Rule of thumb: if you're about to type `HashMap`, stop. Use `BTreeMap`.
//!
//! ---
//!
//! ## 🏠 The rooms in this house (Milestone 1 = build empty rooms)
//!
//! Think of the crate as a house. Right now every room is empty — we're just
//! putting up the walls and labeling the doors. Furniture comes in later
//! milestones.
//!
//! - [`ids`]       — the "coat-check tickets" every object is looked up by.
//! - [`card`]      — one card sitting on the table.
//! - [`effect`]    — what a card actually *does* (the format the engine runs).
//! - [`group`]     — a handful of cards bundled together (ordered!).
//! - [`field`]     — the tabletop: where cards physically sit.
//! - [`processor`] — the engine's heartbeat: a pausable to-do stack.
//! - [`duel`]      — the whole game; the box that owns every other room.

pub mod card;
pub mod duel;
pub mod effect;
pub mod field;
pub mod group;
pub mod ids;
pub mod processor;
pub mod script;
pub mod zone;

/// The two players, by index — named for readability over bare `0` / `1`.
pub const PLAYER_0: usize = 0;
pub const PLAYER_1: usize = 1;
