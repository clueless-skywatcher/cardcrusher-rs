//! A bundle of cards.
//!
//! **Milestone 1:** empty room.
//!
//! **What it's for:** card scripts constantly work with *sets* of cards —
//! "every monster on the field", "the 2 cards I just drew". A `Group` is that
//! bundle.
//!
//! **The determinism catch (house rule #5):** the contents will be a
//! `BTreeSet<CardId>` — an **ordered** set — NOT a `HashSet`. Why? A script that
//! loops over the group must visit cards in the *same* order on every machine, or
//! the game desyncs.
//!
//! ```text
//! HashSet:  {c7, c2, c9}  // order is whatever — different per run 🚫
//! BTreeSet: {c2, c7, c9}  // always sorted — same everywhere ✅
//! ```
#[derive(Debug, Default)]
pub struct Group;
