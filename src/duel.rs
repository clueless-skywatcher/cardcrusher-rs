//! The whole game — the box that owns everything.
//!
//! **Milestone 1:** empty room.
//!
//! **Milestone 2:** `Duel` becomes the "front desk" arena that owns the cards and
//! hands out / looks up / removes them by ticket.
//!
//! **The eventual shape** (fields are added by the milestone that first *reads*
//! one — TDD: no field without a test that uses it):
//!
//! ```text
//! cards:    SlotMap<CardId, Card>      // ✅ M2 — the card arena
//! effects:  SlotMap<EffectId, Effect>  // M5 — the effect/IR arena
//! groups:   SlotMap<GroupId, Group>    // when groups arrive
//! field:    Field                      // later phase — real zones
//! rng:      Xoshiro256StarStar         // when we first shuffle/roll
//! messages: Vec<u8>                    // M4 — outbox: "here's what happened"
//! responses:Vec<u8>                    // M4 — inbox:  the player's answer
//! stack:    Vec<Processor>             // M3 — the to-do notes
//! ```
//!
//! **Key design rule:** no object ever holds a link *back* to the `Duel`.
//! Instead, functions that need the whole game take `&mut Duel` and look things up
//! by ticket. This keeps Rust's borrow checker happy: grab a ticket, look it up,
//! do one small thing, let go.

use slotmap::SlotMap;

use crate::card::Card;
use crate::field::Field;
use crate::ids::CardId;
use crate::processor::{
    DuelMessage, DuelStatus, Processor, MSG_NEW_TURN, MSG_PHASE_BATTLE, MSG_PHASE_DRAW,
    MSG_PHASE_END, MSG_PHASE_MAIN1, MSG_PHASE_MAIN2, MSG_PHASE_STANDBY, MSG_SELECT_CARD,
    MSG_STARTUP,
};
use crate::zone::Zone;

// Roadmap — uncomment each import as the milestone that needs it lands:
// use rand_core::SeedableRng;
// use rand_xoshiro::Xoshiro256StarStar;
// use crate::effect::Effect;
// use crate::field::Field;
// use crate::group::Group;
// use crate::ids::{EffectId, GroupId};
// use crate::processor::Processor;

#[derive(Debug)]
pub struct Duel {
    cards: SlotMap<CardId, Card>,
    // Added by the milestone that first reads each one (see the doc sketch above):
    // effects: SlotMap<EffectId, Effect>,   // M5
    // groups: SlotMap<GroupId, Group>,      // when groups arrive
    field: Field, // later phase
    // rng: Xoshiro256StarStar,               // when we first shuffle/roll
    messages: Vec<DuelMessage>,      // M4 outbox
    responses: Vec<u8>,              // M4 inbox
    processor_stack: Vec<Processor>, // M3 to-do stack
    max_turns: usize,
    turn_hist: Vec<u8>,
    lps: [u32; 2],
}

impl Default for Duel {
    fn default() -> Self {
        Self::new()
    }
}

impl Duel {
    pub fn new() -> Self {
        Duel {
            cards: SlotMap::with_key(),
            // effects: SlotMap::with_key(),
            // groups: SlotMap::with_key(),
            field: Field::new(),
            // rng: Xoshiro256StarStar::seed_from_u64(0),
            messages: Vec::new(),
            responses: Vec::new(),
            processor_stack: Vec::new(),
            max_turns: 10000,
            turn_hist: vec![],
            lps: [8000, 8000],
        }
    }

    pub fn add_card(&mut self, card: Card) -> CardId {
        self.cards.insert(card)
    }

    pub fn get_card(&self, id: CardId) -> Option<&Card> {
        self.cards.get(id)
    }

    pub fn remove_card(&mut self, id: CardId) -> Option<Card> {
        self.cards.remove(id)
    }

    /// Create a card and put it on the bottom of a player's deck.
    pub fn add_to_deck(&mut self, player: usize, card: Card) -> CardId {
        let id = self.cards.insert(card);
        self.field.add_to_deck(player, id);
        id
    }

    /// Draw the top card of a player's deck into their hand.
    pub fn draw(&mut self, player: usize) -> Option<CardId> {
        self.field.draw(player)
    }

    pub fn deck_count(&self, player: usize) -> usize {
        self.field.deck_count(player)
    }

    pub fn hand_count(&self, player: usize) -> usize {
        self.field.hand_count(player)
    }

    pub fn start(&mut self) {
        self.processor_stack.push(Processor::Startup { step: 0 });
    }

    pub fn process(&mut self) -> DuelStatus {
        loop {
            match self.step() {
                DuelStatus::Continue => continue,
                other => return other,
            }
        }
    }

    pub fn messages(&self) -> &[DuelMessage] {
        &self.messages
    }

    /// Run the top task once (the driver loop).
    pub fn step(&mut self) -> DuelStatus {
        // Pop the top task by value first — frees the stack borrow so `run_unit`
        // can push sub-tasks / emit messages through `&mut self`.
        let mut unit = match self.processor_stack.pop() {
            Some(unit) => unit,
            None => return DuelStatus::End, // nothing left → game over
        };

        if self.run_unit(&mut unit) {
            DuelStatus::Continue // finished: drop it (don't push back)
        } else {
            // Paused: put it back (its step was already bumped).
            let is_freeze = unit.needs_answer();
            self.processor_stack.push(unit);
            match is_freeze {
                true => DuelStatus::Awaiting, // needs a human → freeze the duel (M4)
                false => DuelStatus::Continue,
            }
        }
    }

    /// One step of one task. Returns `true` when the task is finished.
    fn run_unit(&mut self, unit: &mut Processor) -> bool {
        match unit {
            Processor::Startup { step } => {
                match step {
                    // Step 0: announce startup, then pause to resume at step 1.
                    // (A real startup would flush a startup event here; none yet.)
                    0 => {
                        self.messages.push(MSG_STARTUP);
                        *step += 1;
                        false
                    }
                    // Last step: hand off to turn 1, then finish.
                    _ => {
                        self.processor_stack
                            .push(Processor::Turn { step: 0, player: 0 });
                        true
                    }
                }
            }
            Processor::Turn { step, player } => {
                if *step == 0 {
                    self.turn_hist.push(*player);
                }
                const PHASES: [DuelMessage; 7] = [
                    MSG_NEW_TURN,
                    MSG_PHASE_DRAW,
                    MSG_PHASE_STANDBY,
                    MSG_PHASE_MAIN1,
                    MSG_PHASE_BATTLE,
                    MSG_PHASE_MAIN2,
                    MSG_PHASE_END,
                ];

                let i = *step as usize;
                self.messages.push(PHASES[i]);
                *step += 1;
                if i + 1 == PHASES.len() {
                    // Switch player
                    if self.turn_hist.len() < self.max_turns {
                        self.processor_stack.push(Processor::Turn {
                            step: 0,
                            player: 1 - *player,
                        });
                    }
                    true
                } else {
                    false
                }
            }
            Processor::SelectCard { step } => match step {
                0 => {
                    self.messages.push(MSG_SELECT_CARD);
                    *step += 1;
                    false
                }
                _ => true,
            },
        }
    }

    pub fn select_card(&mut self) {
        self.processor_stack.push(Processor::SelectCard { step: 0 });
    }

    pub fn set_response(&mut self, response: &[u8]) {
        self.responses.clear();
        self.responses.extend_from_slice(response);
    }

    pub fn set_max_turns(&mut self, turns: usize) {
        self.max_turns = turns
    }

    pub fn turn_history(&self) -> &[u8] {
        &self.turn_hist
    }

    pub fn place(&mut self, card: CardId, zone: Zone) {
        self.field.place(card, zone);
    }

    pub fn zone_of(&self, card: CardId) -> Option<Zone> {
        self.field.zone_of(card)
    }

    pub fn send_to(&mut self, card: CardId, zone: Zone) {
        self.field.place(card, zone);
    }

    pub fn life_points(&self, player: usize) -> u32 {
        self.lps[player]
    }

    pub fn pay_lp(&mut self, player: usize, lp: u32) {
        self.lps[player] = self.lps[player].saturating_sub(lp);
    }

    pub fn deal_damage(&mut self, player: usize, lp: u32) {
        self.lps[player] = self.lps[player].saturating_sub(lp);
    }
}
