//! One card sitting on the table.
//!
//! A `Card` is an **instance** (a physical copy on the board). Its numbers come
//! from [`CardData`] — the static definition harvested from the card's `.lua`
//! script and keyed by `code`. When we make an instance (`Duel::make_card`), we
//! stamp a copy of that definition onto it.
//!
//! The golden rule: anywhere we'd otherwise store a pointer to another card, we
//! store a *ticket* ([`crate::ids::CardId`]) instead. Examples:
//!
//! ```text
//! equipped_to: Option<CardId>   // "the card I'm attached to" — maybe none
//! materials:   Vec<CardId>      // "my Xyz materials" — a list of tickets
//! ```

/// The static definition of a card — its printed numbers, shared by every copy
/// of that `code`. Mirrors EDOPro's `card_data` (`ocgcore/duel.h`): `type`,
/// `attribute`, `race` are **bitmasks** (see the prelude's `TYPE_*`/
/// `ATTRIBUTE_*`/`RACE_*` constants); ATK/DEF are signed so "?" (−2) fits later.
///
/// `text` is our own convenience field — EDOPro keeps names/text in a separate
/// card database, not in the engine core.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CardData {
    /// Card type: `TYPE_MONSTER | TYPE_NORMAL | …` (a bitmask).
    pub card_type: u32,
    /// Attack (signed: −2 means "?").
    pub atk: i32,
    /// Defense (signed: −2 means "?").
    pub def: i32,
    /// Monster level/rank.
    pub level: u32,
    /// Attribute: `ATTRIBUTE_EARTH | …` (a bitmask; monsters have exactly one).
    pub attribute: u32,
    /// Monster Type ("race" in EDOPro): `RACE_WARRIOR | …` (a bitmask).
    pub race: u64,
    /// The card's name/rules text. Ours — not part of EDOPro's card_data.
    pub text: String,
}

#[derive(Debug, Default, Clone)]
pub struct Card {
    /// The card's code — the key into the loaded definitions.
    pub code: u32,
    /// This copy's printed stats (stamped from the definition at creation).
    pub data: CardData,
    /// Battle position — only meaningful while in a Monster Zone (see
    /// [`crate::position::Position`]). Read it via `Duel::position_of`.
    pub position: crate::position::Position,
}

impl Card {
    /// A bare card with just a code and default (zeroed) stats. `Duel::make_card`
    /// is the way to get a card whose stats are filled from its loaded script.
    pub fn new(code: u32) -> Self {
        Card {
            code,
            ..Default::default()
        }
    }

    /// A card with an explicit definition stamped on.
    pub fn with_data(code: u32, data: CardData) -> Self {
        Card {
            code,
            data,
            ..Default::default()
        }
    }
}
