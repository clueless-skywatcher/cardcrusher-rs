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

use std::collections::BTreeMap;

use crate::{ids::CardId, zone::Zone};

#[derive(Debug, Default)]
pub struct Field {
    locations: BTreeMap<CardId, Zone>,
    decks: [Vec<CardId>; 2],
    hands: [Vec<CardId>; 2],
}

impl Field {
    pub fn place(&mut self, card: CardId, zone: Zone) -> Option<Zone> {
        self.locations.insert(card, zone)
    }

    pub fn zone_of(&self, card: CardId) -> Option<Zone> {
        self.locations.get(&card).copied()
    }

    pub fn new() -> Self {
        Field {
            locations: BTreeMap::new(),
            decks: [vec![], vec![]],
            hands: [vec![], vec![]],
        }
    }

    pub fn add_to_deck(&mut self, player: usize, card: CardId) {
        self.decks[player].push(card);
        self.locations.insert(card, Zone::Deck);
    }

    pub fn add_to_hand(&mut self, player: usize, card: CardId) {
        self.hands[player].push(card);
        self.locations.insert(card, Zone::Hand);
    }

    /// Draw the top card of a player's deck into their hand. `None` on an empty
    /// deck (a deck-out — a loss condition to wire up later).
    pub fn draw(&mut self, player: usize) -> Option<CardId> {
        let card = self.decks[player].pop()?; // top of deck = end of the pile
        self.add_to_hand(player, card);
        Some(card)
    }

    pub fn deck_count(&self, player: usize) -> usize {
        self.decks[player].len()
    }

    pub fn hand_count(&self, player: usize) -> usize {
        self.hands[player].len()
    }
}
