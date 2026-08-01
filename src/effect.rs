//! The effect context (a scratchpad) and the Lua verbs that write into it.
//!
//! The `e` a card's stage receives is a plain **Lua table** (see the prelude).
//! Its verbs (`destroy`, `pay_lp`, `targets`) are Lua methods that call the small
//! Rust hooks registered here. Doing it in Lua (not a Rust `UserData`) is what
//! lets a stage `coroutine.yield` mid-run to ask the player.
//!
//! The hooks never touch the `Duel` directly (that would be a borrow cycle) —
//! they record into a shared [`EffectContext`], and the `Duel` applies the
//! records after the stage runs ("describe, then execute").

use std::cell::RefCell;
use std::rc::Rc;

use mlua::Lua;
use slotmap::{Key, KeyData};

use crate::{field::Field, ids::CardId, zone::Zone};

/// Scratchpad shared between the `Duel` and the effect currently resolving.
/// Verbs on `e` write here; the `Duel` reads it back.
#[derive(Default, Debug)]
pub struct EffectContext {
    /// The chosen targets for the resolving effect (set before resolve).
    pub targets: Vec<CardId>,
    /// Cards the script asked to destroy (applied by the `Duel` afterward).
    pub to_destroy: Vec<CardId>,
    /// Cards the script asked to `send` somewhere — `(card, destination)`, applied
    /// by the `Duel` afterward. A plain move, NOT a destruction.
    pub to_move: Vec<(CardId, Zone)>,
    /// Costs the script declared in its `cost` stage. The `Duel` checks they're
    /// all payable, then applies them ("describe, then execute").
    pub costs: Vec<CostType>,
    pub activator: usize,
    pub candidates: Vec<CardId>,
}

/// A cost an effect declares in its `cost` stage. New cost kinds get a variant
/// here plus arms in the duel's `can_pay` / `apply_cost`.
#[derive(Debug, Clone)]
pub enum CostType {
    /// Pay N life points.
    LifePoints(u32),
}

/// How/where an effect is activated. The integer values match the prelude's
/// `ACTIVATE`/`IGNITION`/`QUICK`/`TRIGGER` constants a card passes to
/// `add_effect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    /// A Spell/Trap card's activation — from the hand (or a set S/T zone).
    Activate,
    /// A manual effect on a card you control, on your Main Phase.
    Ignition,
    /// A quick effect — needs the chain/priority engine (not activatable yet).
    Quick,
    /// Fires on an event — needs the event engine (not activatable yet).
    Trigger,
}

impl EffectKind {
    /// Map a prelude kind code to an `EffectKind` (0/unknown → `Activate`).
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => EffectKind::Ignition,
            2 => EffectKind::Quick,
            3 => EffectKind::Trigger,
            _ => EffectKind::Activate,
        }
    }
}

// Card-type bits + subtype values needed to derive spell speed — mirror the
// prelude constants (`card_types.lua` / `spell_types.lua` / `trap_types.lua`).
const TYPE_SPELL: u32 = 0x2;
const TYPE_TRAP: u32 = 0x4;
const SPELL_QUICKPLAY: u32 = 2;
const TRAP_COUNTER: u32 = 3;

/// An effect's **spell speed** (0..3) — a pure function of its `kind` plus the
/// owning card's type + subtype. Mirrors EDOPro `effect::get_speed()`
/// (`effect.cpp:694`):
/// - `Quick` → 2; `Ignition` / `Trigger` → 1;
/// - a Spell/Trap **activation** by subtype: quick-play spell → 2, counter trap →
///   3, otherwise a normal spell → 1 / a normal (or continuous) trap → 2;
/// - anything else (a monster activation) → 0 (non-chainable).
///
/// It gates responses: speed 1 can only *start* a chain, and a response must be
/// `>=` the current top link's speed (see `docs/chain.md`).
pub fn spell_speed(
    kind: EffectKind,
    card_type: u32,
    spell_type: Option<u32>,
    trap_type: Option<u32>,
) -> u8 {
    match kind {
        EffectKind::Quick => 2,
        EffectKind::Ignition | EffectKind::Trigger => 1,
        EffectKind::Activate if card_type & TYPE_TRAP != 0 => {
            if trap_type == Some(TRAP_COUNTER) {
                3
            } else {
                2
            }
        }
        EffectKind::Activate if card_type & TYPE_SPELL != 0 => {
            if spell_type == Some(SPELL_QUICKPLAY) {
                2
            } else {
                1
            }
        }
        EffectKind::Activate => 0,
    }
}

/// Register the effect verbs as VM globals the prelude's `Effect` methods call.
/// Each captures the shared context, so a stage's verbs read/write what the
/// `Duel` sees. One VM per `Duel`, so each hook is bound to that duel's context.
pub fn register_verbs(
    lua: &Lua,
    ctx: Rc<RefCell<EffectContext>>,
    field: Rc<RefCell<Field>>,
) -> mlua::Result<()> {
    // e:targets() -> the chosen targets (card ids, encoded for Lua).
    let c = ctx.clone();
    let targets = lua.create_function(move |_, ()| Ok(encode_ids(&c.borrow().targets)))?;

    // e:destroy(list) -> record those cards to be sent to the GY.
    let c = ctx.clone();
    let destroy = lua.create_function(move |_, ids: Vec<i64>| {
        c.borrow_mut()
            .to_destroy
            .extend(ids.into_iter().map(decode));
        Ok(())
    })?;

    // e:send(list, zone) -> record those cards to be moved to `zone` (a ZONE_*
    // code). A plain relocation, not a destruction — unknown codes are ignored.
    let c = ctx.clone();
    let send = lua.create_function(move |_, (ids, zone): (Vec<i64>, u32)| {
        if let Some(zone) = Zone::from_code(zone) {
            let mut ctx = c.borrow_mut();
            ctx.to_move
                .extend(ids.into_iter().map(|id| (decode(id), zone)));
        }
        Ok(())
    })?;

    // e:pay_lp(n) -> declare a "pay n life points" cost.
    let c = ctx.clone();
    let pay_lp = lua.create_function(move |_, n: u32| {
        c.borrow_mut().costs.push(CostType::LifePoints(n));
        Ok(())
    })?;

    // e:prompt_selection records its candidate set here — for mapping the picked
    // index back to a card, and for the "no legal target" check.
    let c = ctx.clone();
    let prompt_selection = lua.create_function(move |_, ids: Vec<i64>| {
        c.borrow_mut().candidates = ids.into_iter().map(decode).collect();
        Ok(())
    })?;

    // e:monster_zone(who) -> the monsters `who` controls; `who` is relative to
    // the activating player (YOU = same, OPPONENT = the other).
    let c = ctx.clone();
    let monster_zone = lua.create_function(move |_, who: usize| {
        let actual = (who + c.borrow().activator) % 2;
        Ok(encode_ids(&field.borrow().monster_zone(actual)))
    })?;

    lua.globals().set("effect_destroy", destroy)?;
    lua.globals().set("effect_send", send)?;
    lua.globals().set("effect_targets", targets)?;
    lua.globals().set("effect_pay_lp", pay_lp)?;
    lua.globals()
        .set("effect_prompt_selection", prompt_selection)?;
    lua.globals().set("effect_monster_zone", monster_zone)?;

    Ok(())
}

// A `CardId` is an arena ticket Lua can't hold, so we pass it across the boundary
// as its raw 64-bit key value. `as_ffi`/`from_ffi` round-trip losslessly.
fn encode(id: CardId) -> i64 {
    id.data().as_ffi() as i64
}
fn decode(n: i64) -> CardId {
    CardId::from(KeyData::from_ffi(n as u64))
}

/// Encode a slice of card ids into the list Lua sees (e.g. what `e:targets()`
/// returns and what a resumed `prompt_selection` hands back).
pub(crate) fn encode_ids(ids: &[CardId]) -> Vec<i64> {
    ids.iter().map(|id| encode(*id)).collect()
}
