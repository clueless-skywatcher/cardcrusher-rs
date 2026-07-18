//! One card sitting on the table.
//!
//! **Milestone 1:** empty room. It's a real type so the walls connect, but it
//! holds nothing yet.
//!
//! **Coming in Milestone 2:** the actual state — who owns it, where it is, its
//! current ATK/DEF, etc. The golden rule: anywhere we'd otherwise store a
//! pointer to another card, we store a *ticket* ([`crate::ids::CardId`]) instead.
//! Examples:
//!
//! ```text
//! equipped_to: Option<CardId>   // "the card I'm attached to" — maybe none
//! materials:   Vec<CardId>      // "my Xyz materials" — a list of tickets
//! ```
#[derive(Debug, Default)]
pub struct Card;
