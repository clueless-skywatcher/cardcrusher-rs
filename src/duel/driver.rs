//! Player I/O, turn control, and the processor driver loop — the engine's
//! heartbeat: run the top task one step at a time, pausing to ask humans.

use crate::constants::*;
use crate::event::{EVENT_BATTLE_ENDED, EVENT_POST_DAMAGE_CALCULATION};
use crate::ids::CardId;
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

        let unit_run = self.run_unit(&mut unit);
        self.process_events();
        if unit_run {
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
                    self.effect_ctx.borrow_mut().turn_player = *player;
                    // Fresh turn → each player's Normal Summon is available again.
                    self.reset_normal_summons();
                    self.reset_attacks();
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
                } else if PHASES[i] == MSG_PHASE_BATTLE {
                    self.processor_stack.push(Processor::BattleCommand {
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
                            // Normal Summon: gated once per turn. A blocked attempt
                            // just re-shows the menu (no summon, no cost).
                            if let Some(card) = card {
                                if self.can_normal_summon(*player) {
                                    self.summon(card);
                                    self.record_normal_summon(*player);
                                }
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
                        CMD_ACTIVATE => {
                            // The index selects one of the player's currently
                            // activatable effects (kind + location + condition).
                            let opt = self.responses.get(1).copied().unwrap_or(0) as usize;
                            let options = self.activatable_effects(*player);
                            if let Some(&(card, slot)) = options.get(opt) {
                                self.processor_stack.push(Processor::Activate {
                                    step: 0,
                                    card,
                                    slot,
                                    player: *player,
                                });
                            }
                            true
                        }
                        // Anything else keeps us in the Main Phase — re-show.
                        _ => {
                            self.messages.push(MSG_SELECT_IDLECMD);
                            false
                        }
                    }
                }
            },
            Processor::Activate {
                step,
                card,
                slot,
                player,
            } => match step {
                0 => match self.activate(*card, *slot, *player).expect("activate") {
                    DuelStatus::Awaiting => {
                        self.messages.push(MSG_SELECT_CARD);
                        *step += 1;
                        false
                    }
                    // Resolved (or rejected) with no selection → back to the menu.
                    _ => {
                        self.processor_stack.push(Processor::IdleCommand {
                            step: 0,
                            player: *player,
                        });
                        self.processor_stack
                            .push(Processor::ResolveChain { step: 0 });
                        self.processor_stack.push(Processor::ChainResponse {
                            step: 0,
                            player: 1 - *player,
                        });
                        true
                    }
                },
                _ => {
                    let indices = self.responses.iter().map(|&b| b as usize).collect();
                    self.answer_selection(indices);
                    self.resume().expect("resuming");
                    // The effect resolved → return to the Main-Phase menu, so the
                    // player keeps control of the phase until they pass.
                    self.processor_stack.push(Processor::IdleCommand {
                        step: 0,
                        player: *player,
                    });
                    self.processor_stack
                        .push(Processor::ResolveChain { step: 0 });
                    self.processor_stack.push(Processor::ChainResponse {
                        step: 0,
                        player: 1 - *player,
                    });
                    true
                }
            },
            Processor::BattleCommand { step, player } => match step {
                // Step 0: offer the menu, then freeze for a choice.
                0 => {
                    *step += 1;
                    self.messages.push(MSG_SELECT_BATTLECMD);
                    false
                }
                // Step 1+: act on the chosen command. Response = [command, index].
                _ => {
                    let command = self.responses.first().copied().unwrap_or(CMD_NEXT_PHASE);
                    match command {
                        // End the Battle Phase → the menu is done.
                        CMD_NEXT_PHASE => true,
                        // Attack with the chosen attacker (index into `attackers`).
                        CMD_ATTACK => {
                            let idx = self.responses.get(1).copied().unwrap_or(0) as usize;
                            if let Some(&attacker) = self.attackers(*player).get(idx) {
                                self.processor_stack.push(Processor::Attack {
                                    step: 0,
                                    attacker,
                                    player: *player,
                                });
                                true
                            } else {
                                // No such attacker — stay in the menu.
                                self.messages.push(MSG_SELECT_BATTLECMD);
                                false
                            }
                        }
                        // Anything else keeps us in the Battle Phase — re-show.
                        _ => {
                            self.messages.push(MSG_SELECT_BATTLECMD);
                            false
                        }
                    }
                }
            },
            Processor::Attack {
                step,
                attacker,
                player,
            } => match step {
                // Step 0: with opponent monsters, freeze to pick a target;
                // otherwise it's a direct attack — declare it and open the
                // before-damage-calculation window, resuming at step 2 after.
                0 => {
                    if self.attack_targets(*player).is_empty() {
                        self.declare_attack(*attacker, None);
                        self.resume_attack_at(2, *attacker, *player);
                        self.open_before_damage_window(*attacker, None);
                        true
                    } else {
                        self.messages.push(MSG_SELECT_ATTACK_TARGET);
                        *step = 1;
                        false
                    }
                }
                // Step 1: the picked target → declare + open the before-damage
                // window, resuming at step 2 after it closes.
                1 => {
                    let idx = self.responses.first().copied().unwrap_or(0) as usize;
                    let target = self.attack_targets(*player).get(idx).copied();
                    self.declare_attack(*attacker, target);
                    self.resume_attack_at(2, *attacker, *player);
                    self.open_before_damage_window(*attacker, target);
                    true
                }
                // Step 2: the before-damage window has closed. Resolve anything
                // chained there (e.g. Kuriboh), apply the battle, then open the
                // after-damage-calculation window, resuming at step 3 after.
                2 => {
                    if !self.chain.is_empty() {
                        self.resolve_chain();
                    }
                    self.window_timing = None;
                    let (a, target) = self.last_attack.expect("an attack was declared");
                    // A player protected by a NoBattleDamage modifier (e.g. Kuriboh)
                    // takes none of the pending battle damage.
                    let mut dmg = self.effect_ctx.borrow().pending_damage;
                    for (p, d) in dmg.iter_mut().enumerate() {
                        if !self.can_take_battle_damage(p) {
                            *d = 0;
                        }
                    }
                    self.apply_battle_damage(dmg);
                    self.apply_battle_destruction(a, target);
                    self.effect_ctx.borrow_mut().pending_damage = [0, 0];
                    self.resume_attack_at(3, *attacker, *player);
                    self.open_event_window(EVENT_POST_DAMAGE_CALCULATION);
                    true
                }
                // Step 3: the after-damage window has closed → the battle is over.
                // Raise EVENT_BATTLE_ENDED (fires "that battle" subscriptions, e.g.
                // Kuriboh removing its modifier), then back to the menu.
                _ => {
                    if !self.chain.is_empty() {
                        self.resolve_chain();
                    }
                    self.window_timing = None;
                    self.fire_subscriptions(EVENT_BATTLE_ENDED);
                    self.reopen_battle_menu(*player);
                    true
                }
            },
            Processor::OptionalTrigger {
                step,
                effect,
                card,
                player,
                event,
            } => match step {
                0 => {
                    *step += 1;
                    self.messages.push(MSG_SELECT_YESNO);
                    false
                }
                _ => {
                    if self.responses.first().copied() == Some(1) {
                        {
                            let mut ctx = self.effect_ctx.borrow_mut();
                            ctx.activator = *player;
                            ctx.self_card = Some(*card);
                            ctx.event = event.clone(); // firing event's details
                        }
                        let _ = self.resolve_effect(*effect);
                    }
                    true
                }
            },
            Processor::ResolveChain { .. } => {
                self.resolve_chain();
                true
            }
            Processor::ChainResponse { step, player } => match step {
                0 => {
                    self.responder = *player; // whose window this is (for the UI)
                    self.messages.push(MSG_SELECT_CHAIN);
                    *step += 1;
                    false
                }
                _ => {
                    let response = self.responses[0];
                    match response {
                        CMD_PASS => {
                            self.passes[*player] = true;
                            if self.passes[1 - *player] {
                                true
                            } else {
                                self.processor_stack.push(Processor::ChainResponse {
                                    step: 0,
                                    player: 1 - *player,
                                });
                                true
                            }
                        }
                        CMD_RESPONSE => {
                            let effect_index = self.responses[1] as usize;
                            // Works for a chain (spell-speed gated) OR a timing
                            // window with no chain yet (e.g. damage calculation).
                            let (card, slot) = self.response_options_for(*player)[effect_index];
                            // `activate` funnels through `push_chain_link`, which
                            // resets `passes` — no need to reset again here.
                            let _ = self.activate(card, slot, *player);
                            self.processor_stack.push(Processor::ChainResponse {
                                step: 0,
                                player: 1 - *player,
                            });
                            true
                        }
                        _ => true,
                    }
                }
            },
        }
    }

    /// After an attack is declared, reopen the Battle-Phase menu so the player
    /// can attack with another monster (until they choose to move on).
    fn reopen_battle_menu(&mut self, player: usize) {
        self.processor_stack
            .push(Processor::BattleCommand { step: 0, player });
    }

    /// Queue the `Attack` flow to resume at `step` once a pushed sub-window (the
    /// before/after-damage response window) finishes. Call this BEFORE opening the
    /// window, so the window lands on top of the stack and runs first.
    fn resume_attack_at(&mut self, step: u16, attacker: CardId, player: usize) {
        self.processor_stack.push(Processor::Attack {
            step,
            attacker,
            player,
        });
    }

    pub fn chain_length(&self) -> usize {
        self.chain.len()
    }
}
