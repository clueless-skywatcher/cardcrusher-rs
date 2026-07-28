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

mod battle;
mod board;
mod driver;
mod scripting;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use mlua::{Lua, Table, Thread};
use slotmap::SlotMap;

use crate::card::{Card, CardData};
use crate::constants::DuelMessage;
use crate::effect::EffectContext;
use crate::event::DuelEvent;
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
    /// Normal Summons each player has used **this turn** (reset at turn start).
    /// The base rule caps this at 1 — enforced by the Main-Phase menu.
    normal_summons: [u8; 2],
    /// The most recently declared attack: `(attacker, target)`, `None` target =
    /// direct. Set by `declare_attack`; B3 will resolve it into damage.
    last_attack: Option<(CardId, Option<CardId>)>,
    /// Monsters that have already attacked **this turn** (reset at turn start).
    /// Base rule: one attack per monster — enforced in `attackers`.
    attacked_this_turn: BTreeSet<CardId>,
    lps: [u32; 2],
    decked_out: [bool; 2],
    result: Option<Winner>,
    win_reason: Option<WinReason>,

    /// Scripting: the duel OWNS the Lua VM. Registered fns never touch the duel
    /// directly (that would be a borrow cycle) — they share state via `Rc`.
    vm: Lua,
    /// Every effect a loaded card registered, as a Lua object handle. Filled by
    /// the `register_effect` hook that the prelude's `add_effect` calls.
    effects: Rc<RefCell<Vec<(u32, Table)>>>,
    /// Static card definitions harvested from scripts, keyed by `code`. Filled by
    /// the `register_card` hook that the prelude's `Card:new` calls. `BTreeMap`
    /// (not `HashMap`) for deterministic iteration.
    card_data: Rc<RefCell<BTreeMap<u32, CardData>>>,
    effect_ctx: Rc<RefCell<EffectContext>>,

    pending: Option<(Thread, usize, CardId)>,

    events: VecDeque<DuelEvent>,
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
        let card_data = Rc::new(RefCell::new(BTreeMap::new()));
        let effect_ctx = Rc::new(RefCell::new(EffectContext::default()));

        let vm = Lua::new();
        vm.gc_stop(); // determinism: no nondeterministic GC pauses

        Self::set_globals(
            &vm,
            effects.clone(),
            card_data.clone(),
            effect_ctx.clone(),
            field.clone(),
        )
        .expect("failed to set up Lua globals");

        let mut duel = Duel {
            cards: SlotMap::with_key(),
            field,
            messages: Vec::new(),
            responses: Vec::new(),
            processor_stack: Vec::new(),
            max_turns: 10000,
            turn_hist: vec![],
            normal_summons: [0, 0],
            last_attack: None,
            attacked_this_turn: BTreeSet::new(),
            lps: [8000, 8000],
            decked_out: [false, false],
            result: None,
            win_reason: None,
            vm,
            effects,
            card_data,
            effect_ctx,
            pending: None,
            events: VecDeque::new(),
        };
        duel.load_prelude();
        duel
    }

    /// Register the Rust hooks the prelude calls: `register_effect` (how Lua's
    /// `add_effect` hands each effect back to the duel) and the effect verbs
    /// (`e:destroy`/`pay_lp`/`targets`, wired to the shared context).
    fn set_globals(
        vm: &Lua,
        effects: Rc<RefCell<Vec<(u32, Table)>>>,
        card_data: Rc<RefCell<BTreeMap<u32, CardData>>>,
        effect_ctx: Rc<RefCell<EffectContext>>,
        field: Rc<RefCell<Field>>,
    ) -> mlua::Result<()> {
        let register_effect = vm.create_function(move |_, args: (u32, Table)| {
            effects.borrow_mut().push((args.0, args.1));
            Ok(())
        })?;
        vm.globals().set("register_effect", register_effect)?;

        // register_card(code, data) — harvest a card's printed stats from the
        // `data` table its `Card:new` was given. Missing fields default to 0/"".
        let register_card = vm.create_function(move |_, (code, data): (u32, Table)| {
            card_data.borrow_mut().insert(
                code,
                CardData {
                    card_type: data.get("type").unwrap_or(0),
                    atk: data.get("atk").unwrap_or(0),
                    def: data.get("def").unwrap_or(0),
                    level: data.get("level").unwrap_or(0),
                    attribute: data.get("attribute").unwrap_or(0),
                    race: data.get("race").unwrap_or(0),
                    text: data.get("text").unwrap_or_default(),
                    name: data.get("name").unwrap_or_default(),
                },
            );
            Ok(())
        })?;
        vm.globals().set("register_card", register_card)?;

        crate::effect::register_verbs(vm, effect_ctx, field)?;
        Ok(())
    }

    fn load_prelude(&mut self) {
        // Baked into the binary at compile time — no runtime file dependency, so
        // every build runs the byte-identical prelude (determinism). Constant
        // tables load first; the base classes (`base.lua`) load last and may use
        // them. Cards are loaded later still and rely on all of it.
        const PRELUDE: [(&str, &str); 8] = [
            ("players", include_str!("prelude/players.lua")),
            ("effect_kinds", include_str!("prelude/effect_kinds.lua")),
            ("categories", include_str!("prelude/categories.lua")),
            ("card_types", include_str!("prelude/card_types.lua")),
            ("attributes", include_str!("prelude/attributes.lua")),
            ("races", include_str!("prelude/races.lua")),
            ("base", include_str!("prelude/base.lua")),
            ("events", include_str!("prelude/events.lua")),
        ];
        for (name, src) in PRELUDE {
            self.vm
                .load(src)
                .set_name(name)
                .exec()
                .unwrap_or_else(|e| panic!("prelude '{name}' is valid Lua: {e}"));
        }
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
