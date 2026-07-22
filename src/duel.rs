//! The whole game — the box that owns everything: the card arena, the board,
//! the processor stack, the player I/O buffers, and game state (life points,
//! win result).
//!
//! **Design rule:** no object holds a link *back* to the `Duel`. Code that needs
//! the whole game takes `&mut Duel` and looks things up by id — grab a ticket,
//! look it up, do one small thing, let go. This keeps the borrow checker happy.

use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Lua, Table};
use slotmap::SlotMap;

use crate::card::Card;
use crate::constants::*;
use crate::field::Field;
use crate::ids::CardId;
use crate::processor::{DuelStatus, Processor};
use crate::zone::Zone;

pub struct Duel {
    /// Every card in the game, addressed by generational `CardId`.
    cards: SlotMap<CardId, Card>,
    /// The board: zones and per-player piles. Shared (`Rc<RefCell<..>>`) so the
    /// card-scripting layer (built later) can read it live.
    field: Rc<RefCell<Field>>,

    /// Outbox — what the engine has emitted.
    messages: Vec<DuelMessage>,
    /// Inbox — the host's most recent answer.
    responses: Vec<u8>,
    /// The resumable to-do stack.
    processor_stack: Vec<Processor>,

    /// Safety backstop on how many turns run (no real cap in Yu-Gi-Oh!).
    max_turns: usize,
    /// Which player took each turn, in order.
    turn_hist: Vec<usize>,
    lps: [u32; 2],
    decked_out: [bool; 2],
    result: Option<Winner>,
    win_reason: Option<WinReason>,

    /// Scripting: the duel OWNS the Lua VM. Registered fns never touch the duel
    /// directly (that would be a borrow cycle) — they share state via `Rc`.
    vm: Lua,
    /// Every effect a loaded card registered, as a Lua object handle. Filled by
    /// the `register_effect` hook that the prelude's `add_effect` calls.
    effects: Rc<RefCell<Vec<Table>>>,
}

impl Default for Duel {
    fn default() -> Self {
        Self::new()
    }
}

impl Duel {
    // ===== Construction =====================================================

    pub fn new() -> Self {
        let field = Rc::new(RefCell::new(Field::new()));
        let effects = Rc::new(RefCell::new(Vec::new()));

        let vm = Lua::new();
        vm.gc_stop(); // determinism: no nondeterministic GC pauses

        Self::set_globals(&vm, effects.clone()).expect("failed to set up Lua globals");

        let mut duel = Duel {
            cards: SlotMap::with_key(),
            field,
            messages: Vec::new(),
            responses: Vec::new(),
            processor_stack: Vec::new(),
            max_turns: 10000,
            turn_hist: vec![],
            lps: [8000, 8000],
            decked_out: [false, false],
            result: None,
            win_reason: None,
            vm,
            effects,
        };
        duel.load_prelude();
        duel
    }

    /// Register the Rust hooks the prelude calls. `register_effect` is how the
    /// Lua `add_effect` hands each effect object back to the duel to remember.
    fn set_globals(vm: &Lua, effects: Rc<RefCell<Vec<Table>>>) -> mlua::Result<()> {
        let hook = vm.create_function(move |_, eff: Table| {
            effects.borrow_mut().push(eff);
            Ok(())
        })?;
        vm.globals().set("register_effect", hook)?;
        Ok(())
    }

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

    // ===== Card scripting ===================================================

    /// Load a card: run its Lua source. As it runs, the card registers its own
    /// effects (via `Card:new` + `add_effect`).
    pub fn load_card(&mut self, path: &str) -> mlua::Result<()> {
        let src = std::fs::read_to_string(path).map_err(mlua::Error::external)?;
        self.vm.load(&src).exec()
    }

    /// How many effects the loaded cards registered.
    pub fn effect_count(&self) -> usize {
        self.effects.borrow().len()
    }

    fn load_prelude(&mut self) {
        // Baked into the binary at compile time — no runtime file dependency,
        // so every build runs the byte-identical prelude (determinism).
        const PRELUDE: &str = include_str!("prelude.lua");

        self.vm.load(PRELUDE).exec().expect("prelude is valid Lua");
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
