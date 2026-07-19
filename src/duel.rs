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
    MSG_SELECT_IDLECMD, MSG_STARTUP,
};
use crate::zone::Zone;
use crate::{PLAYER_0, PLAYER_1};

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
    turn_hist: Vec<usize>,
    lps: [u32; 2],
    decked_out: [bool; 2],
    result: Option<Winner>,
    win_reason: Option<WinReason>,
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
            decked_out: [false, false],
            result: None,
            win_reason: None,
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

    /// Draw `count` cards off the top of a player's deck into their hand. If the
    /// deck can't supply them all, that player decks out (a loss).
    pub fn draw(&mut self, player: usize, count: usize) -> Vec<CardId> {
        let drawn = self.field.draw(player, count);
        if drawn.len() < count {
            self.decked_out[player] = true;
        }
        self.check_win();
        drawn
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
        // End the match when there is a duel result
        if self.result.is_some() {
            return DuelStatus::End;
        }

        // Pop the top task by value first — frees the stack borrow so `run_unit`
        // can push sub-tasks / emit messages through `&mut self`.
        let mut unit = match self.processor_stack.pop() {
            Some(unit) => unit,
            None => return DuelStatus::End, // nothing left → game over
        };
        // Anything `run_unit` queues lands on top, at indices >= this depth.
        let depth_before = self.processor_stack.len();

        if self.run_unit(&mut unit) {
            DuelStatus::Continue // finished: drop it (don't push back)
        } else {
            // Paused: put it back — but BELOW any sub-tasks it just queued, so
            // those children run first (before this task's next step).
            let is_freeze = unit.needs_answer();
            self.processor_stack.insert(depth_before, unit);
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
                        self.processor_stack.push(Processor::Turn {
                            step: 0,
                            player: PLAYER_0,
                        });
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
                if PHASES[i] == MSG_PHASE_MAIN1 || PHASES[i] == MSG_PHASE_MAIN2 {
                    self.processor_stack.push(Processor::IdleCommand {
                        step: 0,
                        player: *player,
                    });
                    *step += 1;
                    return false;
                }

                *step += 1;
                if i + 1 == PHASES.len() {
                    // Hand over to the other player.
                    if self.turn_hist.len() < self.max_turns {
                        let next = if *player == PLAYER_0 {
                            PLAYER_1
                        } else {
                            PLAYER_0
                        };
                        self.processor_stack.push(Processor::Turn {
                            step: 0,
                            player: next,
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
            Processor::IdleCommand { step, player } => match step {
                // Step 0: offer the menu, then freeze for a choice.
                0 => {
                    *step += 1;
                    self.messages.push(MSG_SELECT_IDLECMD);
                    false
                }
                // Step 1+: act on the chosen command (read from the inbox).
                // Response = [command, index]: 0 = next phase, 1 = summon.
                _ => {
                    let command = self.responses.first().copied().unwrap_or(0);
                    match command {
                        // Go to the next phase → the menu is done.
                        0 => true,
                        // Summon the card at hand slot `index`, then re-show the menu.
                        1 => {
                            let hand_slot = self.responses[1] as usize;
                            if let Some(card) = self.field.hand_card(*player, hand_slot) {
                                self.summon(*player, card);
                            }
                            self.messages.push(MSG_SELECT_IDLECMD);
                            false
                        }
                        // Anything else keeps us in the Main Phase — re-show and wait.
                        _ => {
                            self.messages.push(MSG_SELECT_IDLECMD);
                            false
                        }
                    }
                }
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

    pub fn turn_history(&self) -> &[usize] {
        &self.turn_hist
    }

    pub fn place(&mut self, player: usize, card: CardId, zone: Zone) {
        self.field.place(player, card, zone);
    }

    pub fn zone_of(&self, card: CardId) -> Option<Zone> {
        self.field.zone_of(card)
    }

    pub fn send_to(&mut self, card: CardId, zone: Zone) {
        self.field.send_to(card, zone);
    }

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

    /// Re-evaluate the win conditions from scratch:
    /// a player at 0 LP or decked out has lost. Seeing BOTH players lets us tell a
    /// single loss from a simultaneous draw.
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

        (self.result, self.win_reason) = match (p0_lost, p1_lost) {
            (true, true) => (Some(Winner::Draw), Some(p0_reason)),
            (true, false) => (Some(Winner::Player(PLAYER_1)), Some(p0_reason)),
            (false, true) => (Some(Winner::Player(PLAYER_0)), Some(p1_reason)),
            (false, false) => (None, None),
        };
    }

    pub fn idle_command(&mut self) {
        self.processor_stack.push(Processor::IdleCommand {
            step: 0,
            player: PLAYER_0,
        });
    }

    pub fn summon(&mut self, player: usize, card: CardId) {
        if !self.field.contains(player, card, Zone::Hand) {
            // We should not reach this part of the code
            panic!("Invalid card to choose")
        }
        self.field.send_to(card, Zone::MonsterZone);
    }

    pub fn result(&self) -> Option<Winner> {
        self.result
    }

    pub fn win_reason(&self) -> Option<WinReason> {
        self.win_reason
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Winner {
    Player(usize),
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinReason {
    LifePointsDepleted,
    DeckOut,
    Exodia,
}
