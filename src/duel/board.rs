//! The board & game state: the card arena, deck/hand piles, zones, movement,
//! life points, and win conditions.

use crate::card::Card;
use crate::constants::{PLAYER_0, PLAYER_1};
use crate::ids::CardId;
use crate::zone::Zone;

use super::{Duel, WinReason, Winner};

impl Duel {
    // ===== Card arena =======================================================

    pub fn add_card(&mut self, card: Card) -> CardId {
        self.cards.insert(card)
    }

    pub fn get_card(&self, id: CardId) -> Option<&Card> {
        self.cards.get(id)
    }

    pub fn remove_card(&mut self, id: CardId) -> Option<Card> {
        self.cards.remove(id)
    }

    // ===== Deck & hand ======================================================

    /// Create a card and put it on the bottom of a player's deck.
    pub fn add_to_deck(&mut self, player: usize, card: Card) -> CardId {
        let id = self.cards.insert(card);
        self.field.borrow_mut().add_to_deck(player, id);
        id
    }

    /// Draw `count` cards off the top of a player's deck into their hand. If the
    /// deck can't supply them all, that player decks out (a loss).
    pub fn draw(&mut self, player: usize, count: usize) -> Vec<CardId> {
        let drawn = self.field.borrow_mut().draw(player, count);
        if drawn.len() < count {
            self.decked_out[player] = true;
        }
        self.check_win();
        drawn
    }

    pub fn deck_count(&self, player: usize) -> usize {
        self.field.borrow().deck_count(player)
    }

    pub fn hand_count(&self, player: usize) -> usize {
        self.field.borrow().hand_count(player)
    }

    // ===== Zones & movement =================================================

    pub fn place(&mut self, player: usize, card: CardId, zone: Zone) {
        self.field.borrow_mut().place(player, card, zone);
    }

    pub fn zone_of(&self, card: CardId) -> Option<Zone> {
        self.field.borrow().zone_of(card)
    }

    pub fn send_to(&mut self, card: CardId, zone: Zone) {
        self.field.borrow_mut().send_to(card, zone);
    }

    /// Put a card onto the field as a monster. A shared operation — the menu and
    /// card effects both call it, from any source zone; the caller decides what's
    /// legal.
    pub fn summon(&mut self, card: CardId) {
        self.field.borrow_mut().send_to(card, Zone::MonsterZone);
    }

    /// Set a card face-down in the spell/trap zone. Shared by the menu and card
    /// effects; works regardless of the card's source zone.
    pub fn set_spell_trap(&mut self, card: CardId) {
        self.field.borrow_mut().send_to(card, Zone::SpellTrapZone);
    }

    // ===== Life points & win conditions =====================================

    pub fn life_points(&self, player: usize) -> u32 {
        self.lps[player]
    }

    pub fn pay_lp(&mut self, player: usize, lp: u32) {
        self.lps[player] = self.lps[player].saturating_sub(lp);
        self.check_win();
    }

    pub fn deal_damage(&mut self, player: usize, lp: u32) {
        self.lps[player] = self.lps[player].saturating_sub(lp);
        self.check_win();
    }

    pub fn result(&self) -> Option<Winner> {
        self.result
    }

    pub fn win_reason(&self) -> Option<WinReason> {
        self.win_reason
    }

    /// Re-evaluate the win conditions from scratch: a player at 0 LP or decked
    /// out has lost. Seeing BOTH players lets us tell a single loss from a
    /// simultaneous draw.
    fn check_win(&mut self) {
        let p0_lost = self.lps[PLAYER_0] == 0 || self.decked_out[PLAYER_0];
        let p1_lost = self.lps[PLAYER_1] == 0 || self.decked_out[PLAYER_1];

        // A loser's reason: LP if their life is gone, otherwise deck-out.
        let p0_reason = if self.lps[PLAYER_0] == 0 {
            WinReason::LifePointsDepleted
        } else {
            WinReason::DeckOut
        };
        let p1_reason = if self.lps[PLAYER_1] == 0 {
            WinReason::LifePointsDepleted
        } else {
            WinReason::DeckOut
        };

        match (p0_lost, p1_lost) {
            (true, true) => {
                self.result = Some(Winner::Draw);
                self.win_reason = Some(p0_reason);
            }
            (true, false) => {
                self.result = Some(Winner::Player(PLAYER_1));
                self.win_reason = Some(p0_reason);
            }
            (false, true) => {
                self.result = Some(Winner::Player(PLAYER_0));
                self.win_reason = Some(p1_reason);
            }
            // Nobody is currently losing — leave any decided result untouched. A
            // win is sticky; we never "un-win" (e.g. a future heal above 0 LP).
            (false, false) => {}
        }
    }
}
