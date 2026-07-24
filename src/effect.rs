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

use crate::ids::CardId;

/// Scratchpad shared between the `Duel` and the effect currently resolving.
/// Verbs on `e` write here; the `Duel` reads it back.
#[derive(Default, Debug)]
pub struct EffectContext {
    /// The chosen targets for the resolving effect (set before resolve).
    pub targets: Vec<CardId>,
    /// Cards the script asked to destroy (applied by the `Duel` afterward).
    pub to_destroy: Vec<CardId>,
    /// Life points the script asked to pay (applied by the `Duel` afterward).
    pub lp_payment: u32,
}

/// Register the effect verbs as VM globals the prelude's `Effect` methods call.
/// Each captures the shared context, so a stage's verbs read/write what the
/// `Duel` sees. One VM per `Duel`, so each hook is bound to that duel's context.
pub fn register_verbs(lua: &Lua, ctx: Rc<RefCell<EffectContext>>) -> mlua::Result<()> {
    // e:targets() -> the chosen targets (card ids, encoded for Lua).
    let c = ctx.clone();
    let targets = lua.create_function(move |_, ()| {
        let ids: Vec<i64> = c.borrow().targets.iter().map(|id| encode(*id)).collect();
        Ok(ids)
    })?;
    lua.globals().set("effect_targets", targets)?;

    // e:destroy(list) -> record those cards to be sent to the GY.
    let c = ctx.clone();
    let destroy = lua.create_function(move |_, ids: Vec<i64>| {
        c.borrow_mut()
            .to_destroy
            .extend(ids.into_iter().map(decode));
        Ok(())
    })?;
    lua.globals().set("effect_destroy", destroy)?;

    // e:pay_lp(n) -> record n life points to pay.
    let c = ctx;
    let pay_lp = lua.create_function(move |_, n: u32| {
        c.borrow_mut().lp_payment += n;
        Ok(())
    })?;
    lua.globals().set("effect_pay_lp", pay_lp)?;

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
