//! Player I/O, turn control, and the processor driver loop — the engine's
//! heartbeat: run the top task one step at a time, pausing to ask humans.

use crate::constants::*;
use crate::processor::{DuelStatus, Processor};

use super::Duel;

impl Duel {
    // ===== Player I/O =======================================================

    pub fn messages(&self) -> &[DuelMessage] {
        &self.messages
    }

    pub fn set_response(&mut self, response: &[u8]) {
        self.responses.clear();
        self.responses.extend_from_slice(response);
    }

    /// Queue a stand-alone "pick a card" prompt (used to select an effect target).
    pub fn select_card(&mut self) {
        self.processor_stack.push(Processor::SelectCard { step: 0 });
    }

    /// Queue a stand-alone Main-Phase menu for player 0 (test entry point).
    pub fn idle_command(&mut self) {
        self.idle_command_for(PLAYER_0);
    }

    /// Queue a stand-alone Main-Phase menu for a specific player.
    pub fn idle_command_for(&mut self, player: usize) {
        self.processor_stack
            .push(Processor::IdleCommand { step: 0, player });
    }

    // ===== Turn control =====================================================

    pub fn start(&mut self) {
        self.processor_stack.push(Processor::Startup { step: 0 });
    }

    pub fn set_max_turns(&mut self, turns: usize) {
        self.max_turns = turns;
    }

    pub fn turn_history(&self) -> &[usize] {
        &self.turn_hist
    }

    // ===== Processor driver =================================================

    /// The outer loop: run the top task one step at a time until the stack
    /// drains (`End`) or a task must freeze for a human (`Awaiting`).
    pub fn process(&mut self) -> DuelStatus {
        loop {
            match self.step() {
                DuelStatus::Continue => continue,
                other => return other,
            }
        }
    }

    /// Run the top task once.
    pub fn step(&mut self) -> DuelStatus {
        // A decided game runs nothing more.
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
                true => DuelStatus::Awaiting, // needs a human → freeze the duel
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
                } else if PHASES[i] == MSG_PHASE_DRAW && self.turn_hist.len() > 1 {
                    // Every turn draws except the very first (the opening player
                    // skips their turn-1 draw). turn_hist == [p0] on turn 1.
                    self.draw(*player, 1);
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
                // Step 1+: act on the chosen command. Response = [command, index].
                _ => {
                    let command = self.responses.first().copied().unwrap_or(CMD_NEXT_PHASE);
                    match command {
                        // Go to the next phase → the menu is done.
                        CMD_NEXT_PHASE => true,
                        // Summon the card at hand slot `index`, then re-show.
                        CMD_SUMMON => {
                            let slot = self.responses.get(1).copied().unwrap_or(0) as usize;
                            // Bind first so the field borrow drops before `summon`
                            // (which takes `borrow_mut()`).
                            let card = self.field.borrow().hand_card(*player, slot);
                            if let Some(card) = card {
                                self.summon(card);
                            }
                            self.messages.push(MSG_SELECT_IDLECMD);
                            false
                        }
                        // Set a spell/trap from hand slot `index`, then re-show.
                        CMD_SET_SPELL_TRAP => {
                            let slot = self.responses.get(1).copied().unwrap_or(0) as usize;
                            let card = self.field.borrow().hand_card(*player, slot);
                            if let Some(card) = card {
                                self.set_spell_trap(card);
                            }
                            self.messages.push(MSG_SELECT_IDLECMD);
                            false
                        }
                        // Anything else keeps us in the Main Phase — re-show.
                        _ => {
                            self.messages.push(MSG_SELECT_IDLECMD);
                            false
                        }
                    }
                }
            },
        }
    }
}
