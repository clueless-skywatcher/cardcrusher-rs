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
use crate::ids::CardId;
use crate::processor::{
    DuelMessage, DuelStatus, Processor, MSG_NEW_TURN, MSG_SELECT_CARD, MSG_STARTUP,
};

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
    // field: Field,                          // later phase
    // rng: Xoshiro256StarStar,               // when we first shuffle/roll
    messages: Vec<DuelMessage>,      // M4 outbox
    responses: Vec<u8>,              // M4 inbox
    processor_stack: Vec<Processor>, // M3 to-do stack
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
            // field: Field,
            // rng: Xoshiro256StarStar::seed_from_u64(0),
            messages: Vec::new(),
            responses: Vec::new(),
            processor_stack: Vec::new(),
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
                        self.processor_stack.push(Processor::Turn { step: 0 });
                        true
                    }
                }
            }
            // M3 stub: a turn just announces itself and ends.
            Processor::Turn { .. } => {
                self.messages.push(MSG_NEW_TURN);
                true
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
}
