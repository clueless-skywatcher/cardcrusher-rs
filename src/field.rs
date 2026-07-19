//! The tabletop: where cards physically sit.
//!
//! Two views of location:
//! - `locations` — a card → `(owner, zone)` map: whose zone, and which kind.
//! - per-player **ordered** piles (`decks`, `hands`) — where order matters, e.g.
//!   "draw the top card of player 0's deck".
//!
//! (Unifying these fully — a per-player pile for every zone — is a later cleanup.)

use std::collections::BTreeMap;

use crate::{ids::CardId, zone::Zone};

#[derive(Debug, Default)]
pub struct Field {
    /// Where each card is: its owner and the kind of zone it sits in.
    locations: BTreeMap<CardId, (usize, Zone)>,
    decks: [Vec<CardId>; 2],
    hands: [Vec<CardId>; 2],
}

impl Field {
    pub fn new() -> Self {
        Field {
            locations: BTreeMap::new(),
            decks: [vec![], vec![]],
            hands: [vec![], vec![]],
        }
    }

    /// Record `card` as being in `player`'s `zone`.
    pub fn place(&mut self, player: usize, card: CardId, zone: Zone) {
        self.locations.insert(card, (player, zone));
    }

    pub fn zone_of(&self, card: CardId) -> Option<Zone> {
        self.locations.get(&card).map(|&(_, zone)| zone)
    }

    pub fn owner_of(&self, card: CardId) -> Option<usize> {
        self.locations.get(&card).map(|&(owner, _)| owner)
    }

    /// Move a card to a new zone, keeping its current owner (defaults to player 0
    /// if the card has no recorded owner yet). Removes it from its OLD per-player
    /// pile first, so `deck_count`/`hand_count` stay correct (mirrors EDOPro's
    /// `field::remove_card`, which erases the card from its old zone list).
    pub fn send_to(&mut self, card: CardId, zone: Zone) {
        let owner = self.owner_of(card).unwrap_or(0);
        self.remove_from_pile(card);
        self.locations.insert(card, (owner, zone));
    }

    /// If the card currently lives in a per-player ordered pile (deck/hand), pull
    /// it out. Zones without a pile yet (MZONE/SZONE/GY/…) have nothing to erase.
    fn remove_from_pile(&mut self, card: CardId) {
        let Some(&(owner, zone)) = self.locations.get(&card) else {
            return;
        };
        match zone {
            Zone::Deck => self.decks[owner].retain(|&c| c != card),
            Zone::Hand => self.hands[owner].retain(|&c| c != card),
            _ => {}
        }
    }

    pub fn add_to_deck(&mut self, player: usize, card: CardId) {
        self.decks[player].push(card);
        self.place(player, card, Zone::Deck);
    }

    pub fn add_to_hand(&mut self, player: usize, card: CardId) {
        self.hands[player].push(card);
        self.place(player, card, Zone::Hand);
    }

    /// Draw `count` cards off the top of a player's deck into their hand,
    /// returning them in draw order. Stops early on an empty deck (a deck-out —
    /// a loss condition to wire up later), so the result may be shorter than
    /// `count`.
    pub fn draw(&mut self, player: usize, count: usize) -> Vec<CardId> {
        let mut drawn = Vec::with_capacity(count);
        for _ in 0..count {
            let Some(card) = self.decks[player].pop() else {
                break; // deck-out
            };
            self.add_to_hand(player, card);
            drawn.push(card);
        }
        drawn
    }

    pub fn deck_count(&self, player: usize) -> usize {
        self.decks[player].len()
    }

    pub fn hand_count(&self, player: usize) -> usize {
        self.hands[player].len()
    }

    /// The card at a given slot in a player's hand, if any.
    pub fn hand_card(&self, player: usize, index: usize) -> Option<CardId> {
        self.hands[player].get(index).copied()
    }

    /// Is `card` in `player`'s `zone`? One lookup now that location knows the
    /// owner — works for every zone.
    pub fn contains(&self, player: usize, card: CardId, zone: Zone) -> bool {
        self.locations.get(&card) == Some(&(player, zone))
    }
}
