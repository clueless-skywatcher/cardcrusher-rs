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

use crate::{
    event::{EventDetail, EventSnapshot},
    field::Field,
    ids::CardId,
    modifiers::ModifierType,
    zone::Zone,
};

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
    /// Draws the script asked for — `(player, count)`, applied by the `Duel`
    /// afterward. The player is already absolute (the verb resolves YOU/OPPONENT).
    pub to_draw: Vec<(usize, usize)>,
    /// Costs the script declared in its `cost` stage. The `Duel` checks they're
    /// all payable, then applies them ("describe, then execute").
    pub costs: Vec<CostType>,
    pub activator: usize,
    /// Whose turn it currently is (mirrors the duel's turn player). Lets a verb like
    /// `current_player` answer YOU/OPPONENT relative to the activator.
    pub turn_player: usize,
    pub candidates: Vec<CardId>,
    /// WHICH card instance is running this effect — so verbs like `in_hand`/
    /// `discard_self` act on *this* card, not just its printed code. Set whenever
    /// the engine runs an effect stage for a specific card.
    pub self_card: Option<CardId>,
    /// Battle damage each player is *about to* take at the current damage step —
    /// set before the damage-calc window opens, read by `e:battle_damage()`, then
    /// applied once the window closes.
    pub pending_damage: [u32; 2],
    /// A monotonically increasing counter handing each new modifier a unique id.
    /// Shared (it lives in `ctx`) so a Lua verb can stamp an id and hand it back
    /// synchronously while the `Duel` applies the add afterward.
    pub next_modifier_id: u32,
    /// Player modifiers a verb asked to add — `(id, player, source, kind)`, applied
    /// by the `Duel` after the stage ("describe, then execute").
    pub player_mods_to_add: Vec<(u32, usize, CardId, ModifierType)>,
    /// Modifier ids a verb asked to remove, applied by the `Duel` afterward.
    pub mods_to_remove: Vec<u32>,
    /// Event subscriptions a verb queued, drained onto the `Duel` afterward.
    pub subscriptions_to_add: Vec<Subscription>,
    /// The event that fired the currently-resolving trigger (restored from its
    /// chain link), so `e:get_event_detail` can read its details. `Default`
    /// (code 0) outside a triggered effect.
    pub event: EventSnapshot,
}

/// A standing reaction: run `func` whenever `event` fires. Two flavours, from the
/// two verbs that create one:
/// - `e:queue(event, {count, ..}, fn)` → `remaining: Some(count)`, `until: None` —
///   fires a fixed number of times, then drops. Kuriboh's "undo this once".
/// - `e:apply_event_until(on, until, fn)` → `remaining: None` (unlimited),
///   `until: Some(code)` — repeats until that event arrives. Maxx "C"'s "this turn".
///
/// `owner`/`self_card` are captured **at registration**, and restored before every
/// call. Without them a closure's `YOU` would resolve against whatever effect
/// happened to resolve last.
#[derive(Debug)]
pub struct Subscription {
    pub event: u32,
    /// Firings left; `None` = unlimited (bounded by `until` instead).
    pub remaining: Option<u32>,
    /// The event that removes this subscription; `None` = never expires by event.
    pub until: Option<u32>,
    pub owner: usize,
    pub self_card: Option<CardId>,
    pub func: mlua::Function,
}

/// A cost an effect declares in its `cost` stage. New cost kinds get a variant
/// here plus arms in the duel's `can_pay` / `apply_cost`.
#[derive(Debug, Clone)]
pub enum CostType {
    /// Pay N life points.
    LifePoints(u32),
    /// Discard a specific card (send it from the hand to the GY).
    Discard(CardId),
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
    let f = field.clone();
    let monster_zone = lua.create_function(move |_, who: usize| {
        let actual = (who + c.borrow().activator) % 2;
        Ok(encode_ids(&f.borrow().monster_zone(actual)))
    })?;

    // e:in_hand(who) -> is THIS effect's card in `who`'s hand? `who` is relative to
    // the activator (YOU/OPPONENT). False if we don't know the self card.
    let c = ctx.clone();
    let f = field.clone();
    let in_hand = lua.create_function(move |_, who: usize| {
        let ctx = c.borrow();
        let Some(self_card) = ctx.self_card else {
            return Ok(false);
        };
        let actual = (who + ctx.activator) % 2;
        Ok(f.borrow().contains(actual, self_card, Zone::Hand))
    })?;

    // e:discard_self() -> declare "discard this card" as a cost: the self card is
    // sent to the GY when the cost is paid.
    let c = ctx.clone();
    let discard_self = lua.create_function(move |_, ()| {
        let self_card = c.borrow().self_card;
        if let Some(card) = self_card {
            c.borrow_mut().costs.push(CostType::Discard(card));
        }
        Ok(())
    })?;

    // e:battle_damage() -> the battle damage the activator is about to take at the
    // current damage step (0 outside a damage-calc window).
    let c = ctx.clone();
    let battle_damage = lua.create_function(move |_, ()| {
        let ctx = c.borrow();
        Ok(ctx.pending_damage[ctx.activator])
    })?;

    // e:current_player() -> whose turn it is, relative to the activator: YOU (0) on
    // the activator's own turn, OPPONENT (1) on the other player's turn.
    let c = ctx.clone();
    let current_player = lua.create_function(move |_, ()| {
        let ctx = c.borrow();
        Ok((ctx.turn_player + ctx.activator) % 2)
    })?;

    // e:get_event_detail(code, key) -> the detail `key` of the event that fired this
    // trigger, but only if that event's code == `code` (the code is a guard). A
    // card comes back as its encoded id, a card list as a table of ids. nil if the
    // event doesn't match or has no such detail.
    let c = ctx.clone();
    let get_event_detail = lua.create_function(move |lua, (code, key): (u32, String)| {
        let ctx = c.borrow();
        if ctx.event.code != code {
            return Ok(mlua::Value::Nil);
        }
        Ok(match ctx.event.details.get(&key) {
            Some(detail) => detail_to_lua(lua, detail)?,
            None => mlua::Value::Nil,
        })
    })?;

    // e:add_player_modifier(who, code, value?) -> grant a player modifier and
    // return its new id. `who` is YOU/OPPONENT relative to the activator; the
    // modifier is sourced to this effect's card. The add is applied by the Duel
    // after the stage; the id is stamped now so a closure can capture it.
    let c = ctx.clone();
    let add_player_modifier =
        lua.create_function(move |_, (who, code, value): (usize, u32, Option<i32>)| {
            let mut ctx = c.borrow_mut();
            let Some(source) = ctx.self_card else {
                return Ok(0u32);
            };
            let Some(mod_type) = ModifierType::from_code(code, value.unwrap_or(0)) else {
                return Ok(0u32);
            };
            ctx.next_modifier_id += 1;
            let id = ctx.next_modifier_id;
            let player = (who + ctx.activator) % 2;
            ctx.player_mods_to_add.push((id, player, source, mod_type));
            Ok(id)
        })?;

    // e:remove_modifier(id) -> drop the one modifier with that id (applied after).
    let c = ctx.clone();
    let remove_modifier = lua.create_function(move |_, id: u32| {
        c.borrow_mut().mods_to_remove.push(id);
        Ok(())
    })?;

    // e:queue(event, {count, period}, fn) -> run `fn` when `event` fires, `count`
    // times. (period reserved.) The closure persists on the Duel.
    let c = ctx.clone();
    let queue = lua.create_function(
        move |_, (event, freq, func): (u32, mlua::Table, mlua::Function)| {
            let count = freq.get::<u32>(1).unwrap_or(1);
            let mut ctx = c.borrow_mut();
            let (owner, self_card) = (ctx.activator, ctx.self_card);
            ctx.subscriptions_to_add.push(Subscription {
                event,
                remaining: Some(count),
                until: None,
                owner,
                self_card,
                func,
            });
            Ok(())
        },
    )?;

    // e:apply_event_until(on_event, until_event, fn) -> run `fn` EVERY time
    // `on_event` fires, until `until_event` arrives. A standing rule with an expiry
    // date, not a countdown — that's what "this turn, each time ..." needs.
    let c = ctx.clone();
    let apply_event_until = lua.create_function(
        move |_, (on_event, until_event, func): (u32, u32, mlua::Function)| {
            let mut ctx = c.borrow_mut();
            // Snapshot WHO is setting this up, right now. The rule may not fire for
            // ages, and by then the shared scratchpad will have moved on to some
            // other effect — so we can't look it up later. Grab it while it's true.
            let (owner, self_card) = (ctx.activator, ctx.self_card);
            ctx.subscriptions_to_add.push(Subscription {
                event: on_event,
                remaining: None,          // no limit — the expiry is what stops it
                until: Some(until_event), // ...and this is the expiry
                owner,
                self_card,
                func,
            });
            Ok(())
        },
    )?;

    // e:draw(who, n) -> "draw n cards for who". Records the intent; the Duel does it
    // once the stage finishes, same as every other verb.
    let c = ctx.clone();
    let draw = lua.create_function(move |_, (who, n): (usize, usize)| {
        let mut ctx = c.borrow_mut();
        // Cards say YOU (0) or OPPONENT (1) — relative to whoever owns the effect.
        // Turn that into a real player number here, while we still know who that is.
        // YOU + activator 1 -> player 1. OPPONENT + activator 1 -> player 0.
        let player = (who + ctx.activator) % 2;
        ctx.to_draw.push((player, n));
        Ok(())
    })?;

    lua.globals().set("effect_destroy", destroy)?;
    lua.globals().set("effect_send", send)?;
    lua.globals().set("effect_targets", targets)?;
    lua.globals().set("effect_pay_lp", pay_lp)?;
    lua.globals()
        .set("effect_prompt_selection", prompt_selection)?;
    lua.globals().set("effect_monster_zone", monster_zone)?;
    lua.globals().set("effect_in_hand", in_hand)?;
    lua.globals().set("effect_discard_self", discard_self)?;
    lua.globals().set("effect_battle_damage", battle_damage)?;
    lua.globals().set("effect_current_player", current_player)?;
    lua.globals()
        .set("effect_get_event_detail", get_event_detail)?;
    lua.globals()
        .set("effect_add_player_modifier", add_player_modifier)?;
    lua.globals()
        .set("effect_remove_modifier", remove_modifier)?;
    lua.globals().set("effect_queue", queue)?;
    lua.globals()
        .set("effect_apply_event_until", apply_event_until)?;
    lua.globals().set("effect_draw", draw)?;

    Ok(())
}

/// One event detail as the value Lua sees. Shared by `e:get_event_detail` (one key
/// at a time) and the whole-event table handed to a subscription's closure.
pub(crate) fn detail_to_lua(lua: &Lua, detail: &EventDetail) -> mlua::Result<mlua::Value> {
    Ok(match detail {
        EventDetail::Card(id) => mlua::Value::Integer(encode(*id)),
        EventDetail::Cards(ids) => mlua::Value::Table(lua.create_sequence_from(encode_ids(ids))?),
        EventDetail::Int(n) => mlua::Value::Integer(*n as i64),
        EventDetail::Bool(b) => mlua::Value::Boolean(*b),
    })
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
