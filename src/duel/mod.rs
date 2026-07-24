//! The whole game — the box that owns everything: the card arena, the board,
//! the processor stack, the player I/O buffers, and game state (life points,
//! win result).
//!
//! **Design rule:** no object holds a link *back* to the `Duel`. Code that needs
//! the whole game takes `&mut Duel` and looks things up by id — grab a ticket,
//! look it up, do one small thing, let go. This keeps the borrow checker happy.
//!
//! `impl Duel` is split across sibling files for size (they all see the private
//! fields, being child modules):
//! - [`board`]     — the arena, deck/hand, zones, movement, life points & wins.
//! - [`driver`]    — player I/O, turn control, the processor loop.
//! - [`scripting`] — loading cards and running their Lua effects.

mod board;
mod driver;
mod scripting;

use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Lua, Table};
use slotmap::SlotMap;

use crate::card::Card;
use crate::constants::DuelMessage;
use crate::effect::EffectContext;
use crate::field::Field;
use crate::ids::CardId;
use crate::processor::Processor;

pub struct Duel {
    /// Every card in the game, addressed by generational `CardId`.
    cards: SlotMap<CardId, Card>,
    /// The board: zones and per-player piles. Shared (`Rc<RefCell<..>>`) so the
    /// card-scripting layer can read it live.
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
    effect_ctx: Rc<RefCell<EffectContext>>,
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
        let effect_ctx = Rc::new(RefCell::new(EffectContext::default()));

        let vm = Lua::new();
        vm.gc_stop(); // determinism: no nondeterministic GC pauses

        Self::set_globals(&vm, effects.clone(), effect_ctx.clone())
            .expect("failed to set up Lua globals");

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
            effect_ctx,
        };
        duel.load_prelude();
        duel
    }

    /// Register the Rust hooks the prelude calls: `register_effect` (how Lua's
    /// `add_effect` hands each effect back to the duel) and the effect verbs
    /// (`e:destroy`/`pay_lp`/`targets`, wired to the shared context).
    fn set_globals(
        vm: &Lua,
        effects: Rc<RefCell<Vec<Table>>>,
        effect_ctx: Rc<RefCell<EffectContext>>,
    ) -> mlua::Result<()> {
        let hook = vm.create_function(move |_, eff: Table| {
            effects.borrow_mut().push(eff);
            Ok(())
        })?;
        vm.globals().set("register_effect", hook)?;
        crate::effect::register_verbs(vm, effect_ctx)?;
        Ok(())
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
