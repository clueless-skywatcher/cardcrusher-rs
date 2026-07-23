//! The effect runtime object (`e`) and the scratchpad it writes into.
//!
//! `e` is what every Lua stage receives (`function activate:resolve(e)`). Verbs
//! like `e:destroy(...)` are methods on it. It never touches the `Duel` directly
//! (that would be a borrow cycle) — it **records intent** into a shared
//! [`EffectContext`], and the `Duel` applies those records after the stage runs.

use std::cell::RefCell;
use std::rc::Rc;

use mlua::{UserData, UserDataMethods};
use slotmap::{Key, KeyData};

use crate::ids::CardId;

/// Scratchpad shared between the `Duel` and the effect currently resolving.
/// Verbs on `e` write here; the `Duel` reads it back. "Describe, then execute."
#[derive(Default, Debug)]
pub struct EffectContext {
    /// The chosen targets for the resolving effect (set before resolve).
    pub targets: Vec<CardId>,
    /// Cards the script asked to destroy (applied by the `Duel` afterward).
    pub to_destroy: Vec<CardId>,
    pub lp_payment: u32,
}

/// The `e` object handed to each Lua stage. Holds a clone of the shared context,
/// so its verbs read/write exactly what the `Duel` sees.
pub struct Effect {
    ctx: Rc<RefCell<EffectContext>>,
}

impl Effect {
    pub fn new(ctx: Rc<RefCell<EffectContext>>) -> Self {
        Effect { ctx }
    }
}

impl UserData for Effect {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // e:targets() -> the chosen targets (card ids, encoded for Lua).
        methods.add_method("targets", |_, this, ()| {
            let ids: Vec<i64> = this
                .ctx
                .borrow()
                .targets
                .iter()
                .map(|id| encode(*id))
                .collect();
            Ok(ids)
        });

        // e:destroy(list) -> record those cards to be sent to the GY.
        methods.add_method("destroy", |_, this, ids: Vec<i64>| {
            let cards = ids.into_iter().map(decode);
            this.ctx.borrow_mut().to_destroy.extend(cards);
            Ok(())
        });

        methods.add_method("pay_lp", |_, this, n: u32| {
            this.ctx.borrow_mut().lp_payment += n;
            Ok(())
        });
    }
}

// A `CardId` is an arena ticket Lua can't hold, so we pass it across the boundary
// as its raw 64-bit key value. `as_ffi`/`from_ffi` round-trip losslessly.
fn encode(id: CardId) -> i64 {
    id.data().as_ffi() as i64
}
fn decode(n: i64) -> CardId {
    CardId::from(KeyData::from_ffi(n as u64))
}
