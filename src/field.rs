//! The tabletop: where cards physically sit.
//!
//! **Milestone 1:** empty room.
//!
//! **What it'll be:** the board. Each player gets their zones — monster zones,
//! spell/trap zones, hand, deck, graveyard, banished pile — and the rules about
//! what can go where.
//!
//! ```text
//! Player 0            Player 1
//! ┌───────────────┐   ┌───────────────┐
//! │ hand / deck   │   │ hand / deck   │
//! │ 5 monster     │   │ 5 monster     │
//! │ 5 spell/trap  │   │ 5 spell/trap  │
//! │ graveyard     │   │ graveyard     │
//! └───────────────┘   └───────────────┘
//! ```
//!
//! **Phase 1 note:** we deliberately have NO real zones yet. This is just the
//! labeled room. Zones are a later phase.
#[derive(Debug, Default)]
pub struct Field;
