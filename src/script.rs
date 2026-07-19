//! The card DSL runtime (Rhai). A card is a script; loading it registers effects,
//! each carrying a `resolve` closure the engine runs later.
//!
//! Key constraint: the [`crate::duel::Duel`] now OWNS the Rhai engine, so the
//! registered functions must NOT capture the `Duel` — that would be a reference
//! cycle, and re-borrowing the duel mid-call would panic. Instead they RECORD
//! what the effect wants into a shared [`EffectContext`]; the `Duel` reads that
//! and applies the changes *after* the script runs.

use std::{cell::RefCell, rc::Rc};

use rhai::{Dynamic, Engine, FnPtr, Map};

use crate::ids::CardId;

/// One effect described by a card.
pub struct EffectDef {
    /// Did the card specify a `target`? Decides whether activation asks for one.
    pub has_target: bool,
    /// The `resolve` closure — run later to carry out the effect.
    pub resolve: FnPtr,
}

/// Scratchpad the registered DSL functions write into while a `resolve` runs.
/// The `Duel` sets `targets`, runs the script, then applies `to_destroy`.
#[derive(Default)]
pub struct EffectContext {
    /// The chosen targets for the effect currently resolving.
    pub targets: Vec<CardId>,
    /// Cards the script asked to destroy (applied by the `Duel` afterward).
    pub to_destroy: Vec<CardId>,
    /// Spike observable: how many times `Destroy` was called.
    pub destroys: usize,
}

/// Build a Rhai engine with the card vocabulary registered. The registered
/// functions capture the shared `effects`/`ctx` handles — never the `Duel`.
pub fn build_engine(
    effects: Rc<RefCell<Vec<EffectDef>>>,
    ctx: Rc<RefCell<EffectContext>>,
) -> Engine {
    let mut engine = Engine::new();

    // `RegisterActivate(#{...})`: store an effect from the card's description.
    engine.register_fn("RegisterActivate", move |map: Map| {
        let resolve = map.get("resolve").unwrap().clone().cast::<FnPtr>();
        let has_target = map.contains_key("target");
        effects.borrow_mut().push(EffectDef {
            has_target,
            resolve,
        });
    });

    // `Destroy(what)`: record that the current targets should be destroyed. The
    // Duel applies this after the script returns — no duel access here.
    let ctx_destroy = ctx.clone();
    engine.register_fn("Destroy", move |_what: Dynamic| {
        let mut c = ctx_destroy.borrow_mut();
        let targets = c.targets.clone();
        c.to_destroy.extend(targets);
        c.destroys += 1;
    });

    // Placeholders — exist so cards parse; real meaning later.
    engine.register_fn("PayLP", |_n: i64| Dynamic::UNIT);
    engine.register_fn("Choose", |_a: Dynamic, _b: Dynamic| Dynamic::UNIT);
    engine.register_fn("Monsters", |_a: Dynamic| Dynamic::UNIT);
    engine.register_fn("Exactly", |_n: i64| Dynamic::UNIT);
    engine.register_fn("GetTargets", || Dynamic::UNIT);
    engine.register_fn("Opponent", || Dynamic::UNIT);

    engine
}
