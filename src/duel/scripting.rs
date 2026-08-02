//! Loading cards and running their Lua effect stages.
//!
//! An effect stage (`cost`/`resolve`/…) is a Lua method. We call it with the
//! effect table as both `self` and `e`; its verbs record intents into the shared
//! context, and we apply those to the real duel ("describe, then execute").

use mlua::thread::ThreadStatus;

use crate::chain::ChainLink;
use crate::effect::{spell_speed, CostType, EffectKind, Subscription};
use crate::event::{DuelEvent, EventSnapshot};
use crate::ids::CardId;
use crate::modifiers::{Modifier, ModifierType};
use crate::processor::{DuelStatus, Processor};
use crate::zone::Zone;

use super::Duel;

impl Duel {
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

    pub fn set_targets(&mut self, targets: Vec<CardId>) {
        self.effect_ctx.borrow_mut().targets = targets;
    }

    /// Run the effect's `cost` stage and commit the declared costs — but only if
    /// the player can afford ALL of them. Returns `false` (paying nothing) if any
    /// is unpayable, so activation can be rejected.
    pub fn pay_cost(&mut self, effect_idx: usize, player: usize) -> mlua::Result<bool> {
        let effect_table = self.effects.borrow()[effect_idx].1.clone();
        let cost_func = effect_table.get::<mlua::Function>("cost")?;
        // The effect table is both `self` and `e` — its verbs declare the cost(s).
        cost_func.call::<()>((effect_table.clone(), effect_table))?;

        // Take the declared costs out; commit only if every one is payable.
        let costs: Vec<CostType> = self.effect_ctx.borrow_mut().costs.drain(..).collect();
        if !costs.iter().all(|cost| self.can_pay(cost, player)) {
            return Ok(false);
        }
        for cost in &costs {
            self.apply_cost(cost, player);
        }
        Ok(true)
    }

    /// Whether `player` can currently pay `cost`.
    fn can_pay(&self, cost: &CostType, player: usize) -> bool {
        match cost {
            // EDOPro `check_lp_cost`: payable iff the cost is ≤ their LP (paying
            // down to exactly 0 is legal).
            CostType::LifePoints(n) => self.life_points(player) >= *n,
            // Discard is payable while the card is actually in the hand.
            CostType::Discard(card) => self.zone_of(*card) == Some(Zone::Hand),
        }
    }

    /// Apply a cost that has already been verified payable.
    fn apply_cost(&mut self, cost: &CostType, player: usize) {
        match cost {
            CostType::LifePoints(n) => self.pay_lp(player, *n),
            // A discard is a plain send (hand → GY), NOT a destruction.
            CostType::Discard(card) => self.send_to(*card, Zone::GY),
        }
    }

    pub fn resolve_effect(&mut self, effect_idx: usize) -> mlua::Result<()> {
        let effect_table = self.effects.borrow()[effect_idx].1.clone();
        let resolve_func = effect_table.get::<mlua::Function>("resolve")?;
        resolve_func.call::<()>((effect_table.clone(), effect_table))?;

        self.drain_effect_ops();
        Ok(())
    }

    /// Apply **everything** a script just recorded into `ctx`. The single drain
    /// point: used after a `resolve` stage and after every subscription closure, so
    /// a queued closure's destroys/moves/draws can't leak into the next effect.
    fn drain_effect_ops(&mut self) {
        self.handle_destroys();
        self.handle_moves();
        self.handle_draws();
        self.apply_script_ops();
    }

    /// Apply the modifier/subscription intents a stage's verbs recorded into `ctx`
    /// ("describe, then execute"): grant player modifiers, remove by id, and enrol
    /// any queued event subscriptions onto the duel.
    fn apply_script_ops(&mut self) {
        let adds: Vec<(u32, usize, CardId, ModifierType)> = self
            .effect_ctx
            .borrow_mut()
            .player_mods_to_add
            .drain(..)
            .collect();
        for (id, player, source, mod_type) in adds {
            self.player_modifiers[player].push(Modifier {
                id,
                source,
                mod_type,
            });
        }

        let removes: Vec<u32> = self
            .effect_ctx
            .borrow_mut()
            .mods_to_remove
            .drain(..)
            .collect();
        for id in removes {
            self.remove_modifier(id);
        }

        let subs: Vec<Subscription> = self
            .effect_ctx
            .borrow_mut()
            .subscriptions_to_add
            .drain(..)
            .collect();
        self.subscriptions.extend(subs);
    }

    /// Announce that something happened, with no card attached — "a battle ended",
    /// "the turn ended". It joins the same queue as a destroy, so `process_events`
    /// hands it to subscriptions on the next drain.
    pub fn raise_event(&mut self, code: u32) {
        self.events.push_back(DuelEvent::global(code));
    }

    /// Raise `event` at the subscriptions: **fire, then expire.**
    ///
    /// 1. Every subscription listening for this event runs once. Ones with a count
    ///    tick down and drop at zero; unlimited ones stay.
    /// 2. *Then* every subscription whose `until` is this event is removed — each
    ///    getting one final call as a teardown hook.
    ///
    /// The two steps are ordered so a rule can still act on the very event that
    /// kills it (an End-Phase effect sees a "this turn" rule that expires at turn
    /// end), and so a rule *registered during* step 1 still dies in step 2. Mirrors
    /// EDOPro running `PointEvent` (processor.cpp:557) before `reset_phase` (`:563`).
    pub fn fire_subscriptions(&mut self, event: &EventSnapshot) {
        // ============ STEP 1: FIRE ============

        // Lift the whole list off the Duel. We're about to run Lua closures that can
        // add MORE subscriptions, and they'd be writing to this same list while we
        // walk it. Taking it out means we own a private copy and they can't trip us.
        let subs = std::mem::take(&mut self.subscriptions);

        // Split into two piles: ones waiting on THIS event, and everyone else.
        // `partition` gives back (matches, non-matches) in one pass.
        let (fired, mut keep): (Vec<Subscription>, Vec<Subscription>) =
            subs.into_iter().partition(|s| s.event == event.code);

        for mut sub in fired {
            // `false` = this is a normal firing, not the teardown call.
            self.run_subscription(&sub, event, false);

            // Did that use up its last firing? Two kinds of subscription:
            let spent = match sub.remaining {
                // Counted (from `queue`) — tick down. Kuriboh starts at 1, so one
                // firing takes it to 0 and it's done.
                Some(n) => {
                    let left = n.saturating_sub(1);
                    sub.remaining = Some(left);
                    left == 0
                }
                // Unlimited (from `apply_event_until`) — Maxx "C" draws as many
                // times as the opponent summons. Only its `until` event stops it.
                None => false,
            };

            // Still has life in it → put it back in the keep pile.
            if !spent {
                keep.push(sub);
            }
        }

        // Closures we just ran may have registered new subscriptions — those landed
        // on the (currently empty) real list. Scoop them up behind the survivors so
        // nothing gets dropped, then put everything back on the Duel.
        keep.append(&mut self.subscriptions);
        self.subscriptions = keep;

        // ============ STEP 2: EXPIRE ============
        // Deliberately after step 1, and deliberately over the list as it stands NOW
        // (which includes anything step 1 just registered). That's what makes a
        // "this turn" rule created during the End Phase die on the spot instead of
        // surviving into next turn.

        let subs = std::mem::take(&mut self.subscriptions);

        // This time split on `until` — "which subscriptions does this event KILL?"
        let (expired, keep): (Vec<Subscription>, Vec<Subscription>) =
            subs.into_iter().partition(|s| s.until == Some(event.code));

        // Put the survivors back first. The expiring ones are already out of the
        // list, so their final call can't accidentally re-fire them.
        self.subscriptions = keep;

        for sub in expired {
            // `true` = teardown call. The closure gets the event in its SECOND slot,
            // which is how a card tells "I'm being removed" from "do the thing".
            self.run_subscription(&sub, event, true);
        }
    }

    /// Run one subscription's closure. Restores the registering effect's context
    /// first (`activator`/`self_card`/`event`) so relative verbs like `e:draw(YOU,..)`
    /// still mean the right player, then drains whatever it recorded.
    ///
    /// The closure is called as `fn(ev, until_ev)` with exactly one non-nil: `ev`
    /// on a normal firing, `until_ev` when the subscription is being removed.
    fn run_subscription(&mut self, sub: &Subscription, event: &EventSnapshot, expiring: bool) {
        {
            // Put back WHO this rule belongs to before the closure runs. This is the
            // whole reason `Subscription` carries an owner: the scratchpad is shared,
            // so by now `activator` holds whatever effect resolved most recently —
            // probably the opponent's. Without this line, Maxx "C"'s `draw(YOU, 1)`
            // could hand the card to the wrong player.
            let mut ctx = self.effect_ctx.borrow_mut();
            ctx.activator = sub.owner;
            ctx.self_card = sub.self_card;
            ctx.event = event.clone(); // so `e:get_event_detail` works in here too
        } // borrow ends here — the Lua call below needs the scratchpad free

        let table = self.event_table(event);

        // The closure signature is fn(ev, until_ev). Exactly one is filled in, and
        // WHICH one tells the card what kind of call this is:
        let args = match table {
            // teardown: "you're being removed" → second slot
            Some(t) if expiring => (mlua::Value::Nil, mlua::Value::Table(t)),
            // normal firing: "the thing happened" → first slot
            Some(t) => (mlua::Value::Table(t), mlua::Value::Nil),
            // couldn't build the table (shouldn't happen) — still call it, empty
            None => (mlua::Value::Nil, mlua::Value::Nil),
        };

        // Ignore a Lua error here: one broken card shouldn't kill the whole duel.
        let _ = sub.func.call::<()>(args);

        // The closure only WROTE DOWN what it wants (destroy/move/draw/modifiers).
        // Nothing has happened yet — this is the line that actually does it.
        self.drain_effect_ops();
    }

    /// A whole event as a Lua table: `code` plus every detail under its own key.
    /// So `ev.code` and `ev.destroyed_card` both just work on the card side.
    fn event_table(&self, event: &EventSnapshot) -> Option<mlua::Table> {
        let table = self.vm.create_table().ok()?;
        table.set("code", event.code).ok()?;
        // `details` is a BTreeMap, so this walk is always in the same order —
        // determinism rule 5. A Lua table built from a HashMap could differ per run.
        for (key, detail) in &event.details {
            let value = crate::effect::detail_to_lua(&self.vm, detail).ok()?;
            table.set(key.as_str(), value).ok()?;
        }
        Some(table)
    }

    // ===== M4/M5: the coroutine bridge ======================================

    /// Activate `card`'s effect in `slot`, as `player`. In order:
    /// - fail if the card has no such effect;
    /// - fail if the effect's `condition` returns false (no cost, no freeze);
    /// - run the `target` stage on a Lua coroutine to learn the candidate set;
    ///   with legal candidates → pay cost, freeze (`Awaiting`) for the pick; an
    ///   empty candidate set → reject up front (no cost); no selection asked →
    ///   pay cost and resolve immediately.
    ///
    /// Cost is paid only once the activation is committed — never on a rejection.
    pub fn activate(
        &mut self,
        card: CardId,
        slot: usize,
        player: usize,
    ) -> mlua::Result<DuelStatus> {
        let idx = match self.effect_index(card, slot) {
            Some(i) => i,
            None => return Ok(DuelStatus::End),
        };
        let effect = self.effects.borrow()[idx].1.clone();
        // A Spell/Trap card activation (Activate kind) has a card lifecycle: it
        // moves to the field on activation and to the GY after it resolves.
        let is_spell = self.effect_kind(&effect) == EffectKind::Activate;

        // Set the activator + self card BEFORE `condition` — a condition can ask
        // about YOU/OPPONENT (relative to the activating player) or about THIS
        // card's own location (`in_hand`).
        {
            let mut ctx = self.effect_ctx.borrow_mut();
            ctx.activator = player;
            ctx.self_card = Some(card);
        }
        if !self.check_condition(&effect)? {
            return Ok(DuelStatus::End);
        }

        let target_func = effect.get::<mlua::Function>("target")?;
        let thread = self.vm.create_thread(target_func)?;
        thread.resume::<mlua::Value>((effect.clone(), effect))?;

        match thread.status() {
            // The target stage yielded — it wants a selection.
            ThreadStatus::Resumable => {
                if self.effect_ctx.borrow().candidates.is_empty() {
                    // No legal target → activation is rejected; cost is NOT paid.
                    return Ok(DuelStatus::End);
                }
                if !self.pay_cost(idx, player)? {
                    // Can't pay the cost → activation is rejected.
                    return Ok(DuelStatus::End);
                }
                // The activated Spell/Trap goes to the field (EDOPro AddChain).
                if is_spell {
                    self.send_to(card, Zone::SpellTrapZone);
                }
                self.pending = Some((thread, idx, card));
                Ok(DuelStatus::Awaiting)
            }
            // No selection needed — pay cost and resolve straight away.
            _ => {
                if !self.pay_cost(idx, player)? {
                    return Ok(DuelStatus::End);
                }
                if is_spell {
                    self.send_to(card, Zone::SpellTrapZone);
                }
                let targets = self.effect_ctx.borrow().targets.clone();
                // A plain activation isn't fired by an event → no snapshot.
                self.push_chain_link(idx, card, player, targets, EventSnapshot::default());
                Ok(DuelStatus::End)
            }
        }
    }

    /// The candidate cards offered by the effect currently awaiting a selection
    /// (what a `MSG_SELECT_CARD` prompt is asking you to pick from).
    pub fn candidates(&self) -> Vec<CardId> {
        self.effect_ctx.borrow().candidates.clone()
    }

    /// Pick the effect's targets by index into the offered candidate set (what
    /// `prompt_selection` was given).
    pub fn answer_selection(&mut self, indices: Vec<usize>) {
        let targets: Vec<CardId> = {
            let ctx = self.effect_ctx.borrow();
            indices.iter().map(|&i| ctx.candidates[i]).collect()
        };
        self.set_targets(targets);
    }

    /// Resume the frozen effect: hand the chosen cards back to the paused
    /// `prompt_selection` (so it *returns* them), let the `target` stage finish,
    /// then resolve the effect.
    pub fn resume(&mut self) -> mlua::Result<DuelStatus> {
        let (thread, index, card) = self
            .pending
            .take()
            .expect("nothing is awaiting a selection");
        let chosen = crate::effect::encode_ids(&self.effect_ctx.borrow().targets);
        thread.resume::<mlua::Value>(chosen)?;
        let activator = self.effect_ctx.borrow().activator;
        let targets = self.effect_ctx.borrow().targets.clone();
        self.push_chain_link(index, card, activator, targets, EventSnapshot::default());
        Ok(DuelStatus::End)
    }

    /// Add a link to the chain and reset the response-window pass tracking. Every
    /// new link (link 1 or a chained response) re-opens the window for both
    /// players, so `passes` goes back to `[false, false]`. Funnel all chain adds
    /// through here so the reset can't be forgotten.
    fn push_chain_link(
        &mut self,
        effect_seq: usize,
        card: CardId,
        activator: usize,
        targets: Vec<CardId>,
        event: EventSnapshot,
    ) {
        self.chain.push(ChainLink {
            effect_seq,
            card,
            activator,
            targets,
            event,
        });
        self.passes = [false, false];
    }

    pub fn code_effects(&self, code: u32) -> Vec<mlua::Table> {
        self.effects
            .borrow()
            .iter()
            .filter(|(c, _)| *c == code)
            .map(|(_, t)| t.clone())
            .collect()
    }

    pub fn effects_of(&self, card: CardId) -> Vec<mlua::Table> {
        let card = self.get_card(card);
        match card {
            Some(card) => self.code_effects(card.code),
            None => Vec::new(),
        }
    }

    /// The effects `player` can activate right now, as `(card, effect slot)`:
    /// `Activate` effects on cards in their hand, and `Ignition` effects on
    /// monsters they control — each with a passing `condition`. (`Quick`/`Trigger`
    /// need the chain/event engine and are never offered here yet.)
    pub fn activatable_effects(&self, player: usize) -> Vec<(CardId, usize)> {
        let hand: Vec<CardId> = {
            let f = self.field.borrow();
            (0..f.hand_count(player))
                .filter_map(|i| f.hand_card(player, i))
                .collect()
        };
        let monsters = self.field.borrow().monster_zone(player);

        let mut out = Vec::new();
        for card in hand {
            self.collect_activatable(card, EffectKind::Activate, player, &mut out);
        }
        for card in monsters {
            self.collect_activatable(card, EffectKind::Ignition, player, &mut out);
        }
        out
    }

    /// The spell speed (0..3) of a chain link — its effect's kind + owning card's
    /// type/subtype, fed to `spell_speed`. Takes `&ChainLink` so a caller can pass
    /// `self.chain.last()` straight in. Used to gate responses against the top link.
    pub fn speed_of(&self, link: &ChainLink) -> u8 {
        let Some(data) = self.card_data(link.card) else {
            return 0; // card gone → treat as non-chainable
        };
        let kind = self.effect_kind(&self.effects.borrow()[link.effect_seq].1);
        spell_speed(kind, data.card_type, data.spell_type, data.trap_type)
    }

    /// The effects `player` may activate **in response** to the current chain: the
    /// `activatable_effects` checks plus the spell-speed gate (`>= 2` and `>= the
    /// top link's speed`). `chain_link` is the top link. Hand `ACTIVATE` only for
    /// now — field `QUICK` effects (also speed 2) join here next rung; `IGNITION`
    /// (speed 1) can never chain, so there's no monster loop yet.
    pub fn chainable_effects(&self, player: usize, chain_link: &ChainLink) -> Vec<(CardId, usize)> {
        let top_speed = self.speed_of(chain_link); // constant across all candidates
        let hand: Vec<CardId> = {
            let f = self.field.borrow();
            (0..f.hand_count(player))
                .filter_map(|i| f.hand_card(player, i))
                .collect()
        };

        let mut out = Vec::new();
        for card in hand {
            self.collect_chainable(top_speed, card, EffectKind::Activate, player, &mut out);
        }
        out
    }

    /// Append `(card, slot)` for each of `card`'s effects of kind `want` whose
    /// `condition` currently passes.
    fn collect_activatable(
        &self,
        card: CardId,
        want: EffectKind,
        player: usize,
        out: &mut Vec<(CardId, usize)>,
    ) {
        {
            let mut ctx = self.effect_ctx.borrow_mut();
            ctx.activator = player;
            ctx.self_card = Some(card);
        }
        for (slot, effect) in self.effects_of(card).iter().enumerate() {
            if self.effect_kind(effect) == want
                && self.check_condition(effect).unwrap_or(false)
                && self.has_legal_target(effect)
            {
                out.push((card, slot));
            }
        }
    }

    /// Like `collect_activatable`, but keeps only effects whose spell speed passes
    /// the response gate: `>= 2` (SS1 never responds) and `>= top_speed`.
    fn collect_chainable(
        &self,
        top_speed: u8,
        card: CardId,
        want: EffectKind,
        player: usize,
        out: &mut Vec<(CardId, usize)>,
    ) {
        let Some(data) = self.card_data(card) else {
            return;
        };
        {
            let mut ctx = self.effect_ctx.borrow_mut();
            ctx.activator = player;
            ctx.self_card = Some(card);
        }
        for (slot, effect) in self.effects_of(card).iter().enumerate() {
            let kind = self.effect_kind(effect);
            let speed = spell_speed(kind, data.card_type, data.spell_type, data.trap_type);
            if speed >= 2
                && speed >= top_speed
                && kind == want
                && self.check_condition(effect).unwrap_or(false)
                && self.has_legal_target(effect)
            {
                out.push((card, slot));
            }
        }
    }

    /// Whether this effect's `target` stage finds at least one legal candidate.
    /// Runs it on a scratch coroutine (read-only probe): if it yields asking for a
    /// selection, the candidate set must be non-empty; if it never asks, there's
    /// nothing to target → always activatable (e.g. Pot of Greed).
    fn has_legal_target(&self, effect: &mlua::Table) -> bool {
        let Ok(target_func) = effect.get::<mlua::Function>("target") else {
            return true;
        };
        let Ok(thread) = self.vm.create_thread(target_func) else {
            return true;
        };
        self.effect_ctx.borrow_mut().candidates.clear();
        if thread
            .resume::<mlua::Value>((effect.clone(), effect.clone()))
            .is_err()
        {
            return true;
        }
        match thread.status() {
            ThreadStatus::Resumable => !self.effect_ctx.borrow().candidates.is_empty(),
            _ => true,
        }
    }

    /// An effect's declared kind (read from its Lua table; defaults to Activate).
    pub fn effect_kind(&self, effect: &mlua::Table) -> EffectKind {
        EffectKind::from_code(effect.get::<u32>("kind").unwrap_or(0))
    }

    fn handle_destroys(&mut self) {
        let to_destroy: Vec<CardId> = self.effect_ctx.borrow_mut().to_destroy.drain(..).collect();
        for card in to_destroy {
            // An effect's `e:destroy` is destruction by effect — same chokepoint
            // as battle, tagged with its own reason.
            self.destroy(card, crate::reason::REASON_EFFECT);
        }
    }

    /// Apply the effect's `draw` intents.
    fn handle_draws(&mut self) {
        // Take the list OUT of the scratchpad first. `self.draw` needs `&mut self`,
        // and we'd still be holding a borrow of the scratchpad if we looped over it
        // in place — Rust would refuse. Copy out, let go, then act.
        let to_draw: Vec<(usize, usize)> = self.effect_ctx.borrow_mut().to_draw.drain(..).collect();
        for (player, count) in to_draw {
            // The verb already turned YOU/OPPONENT into a real player number, so
            // this is just "give player N their cards". `draw` also handles the
            // deck-out loss if there aren't enough left.
            self.draw(player, count);
        }
    }

    /// Apply the effect's `send` intents — plain relocations, NOT destructions
    /// (no reason stamped, no `EVENT_DESTROYED`).
    fn handle_moves(&mut self) {
        let to_move: Vec<(CardId, Zone)> = self.effect_ctx.borrow_mut().to_move.drain(..).collect();
        for (card, zone) in to_move {
            self.send_to(card, zone);
        }
    }

    fn effect_index(&self, card: CardId, slot: usize) -> Option<usize> {
        let code = self.get_card(card)?.code;
        self.effects
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, (c, _))| *c == code)
            .nth(slot)
            .map(|(i, _)| i)
    }

    fn check_condition(&self, effect: &mlua::Table) -> mlua::Result<bool> {
        let cond = effect.get::<mlua::Function>("condition")?;
        cond.call::<bool>((effect.clone(), effect.clone()))
    }

    /// Drain the event queue: every fired TRIGGER goes ON THE CHAIN (C4), and when
    /// several fire at once they're ordered by **SEGOC** (C5) — collect all fired
    /// mandatory triggers, sort them turn-player-first, then build the links. (See
    /// `docs/segoc.md`.) Optionals still resolve via the inline `OptionalTrigger`
    /// yes/no; SEGOC-ordering them is deferred.
    pub fn process_events(&mut self) {
        let turn_player = self.turn_hist.last().copied().unwrap_or(0);
        // (effect, card, controller, the event that fired it).
        let mut fired: Vec<(usize, CardId, usize, EventSnapshot)> = Vec::new();

        while let Some(event) = self.events.pop_front() {
            // Freeze the event's details right now. The trigger it fires might not
            // resolve for a while (several chain links deep), and by then this event
            // is long gone from the queue — so the link carries a copy.
            let snapshot = EventSnapshot {
                code: event.code,
                details: event.details,
            };

            // Subscriptions go FIRST, before any trigger is placed on the chain.
            // They're the "immediately, without using the chain" lane — a Maxx "C"
            // draw isn't something the opponent can respond to. (EDOPro splits the
            // same two lanes in `process_instant_event`, processor.cpp:1173.)
            self.fire_subscriptions(&snapshot);

            // Now the trigger lane. A global event (a battle ending, a turn ending)
            // has no card, so nothing can have a "when THIS card ..." trigger on it
            // — subscriptions were the whole point of raising it.
            let Some(card_id) = event.card else {
                continue;
            };
            // Card's gone (already left the field)? Nothing to hang a trigger on.
            let Some(card) = self.get_card(card_id) else {
                continue;
            };
            let card_code = card.code;
            let player = self.controller_of(card_id);
            let indexes: Vec<usize> = self
                .effects
                .borrow()
                .iter()
                .enumerate()
                .filter(|(_, (code, t))| {
                    *code == card_code
                        && self.effect_kind(t) == EffectKind::Trigger
                        && t.get::<u32>("event").unwrap_or(0) == event.code
                })
                .map(|(i, _)| i)
                .collect();

            for idx in indexes {
                self.effect_ctx.borrow_mut().activator = player;
                let t = self.effects.borrow()[idx].1.clone();
                if !self.check_condition(&t).unwrap_or(false) {
                    continue;
                }
                if t.get::<bool>("optional").unwrap_or(false) {
                    self.processor_stack.push(Processor::OptionalTrigger {
                        step: 0,
                        effect: idx,
                        card: card_id,
                        player,
                        event: snapshot.clone(),
                    });
                } else {
                    fired.push((idx, card_id, player, snapshot.clone()));
                }
            }
        }

        // SEGOC: the turn player's triggers are placed first. A STABLE sort keeps
        // each player's triggers in event order (our stand-in for "the player
        // chooses their own order"). No targets yet — targeting triggers deferred.
        fired.sort_by_key(|(_, _, player, _)| *player != turn_player);
        let any_fired = !fired.is_empty();
        for (idx, card, player, snapshot) in fired {
            self.push_chain_link(idx, card, player, Vec::new(), snapshot);
        }

        // Any links built → resolve them through the chain: open the response
        // window (opponent of the turn player responds first), then ResolveChain
        // unwinds LIFO — the same machinery as an activation.
        if any_fired {
            self.processor_stack
                .push(Processor::ResolveChain { step: 0 });
            self.processor_stack.push(Processor::ChainResponse {
                step: 0,
                player: 1 - turn_player,
            });
        }
    }

    /// The controller of each link on the chain, in chain order (link 1 first).
    /// Used to observe SEGOC placement — see `docs/segoc.md`.
    pub fn chain_activators(&self) -> Vec<usize> {
        self.chain.iter().map(|l| l.activator).collect()
    }

    /// The player whose chain-response window is currently open.
    pub fn chain_responder(&self) -> usize {
        self.responder
    }

    /// The effects the current responder may activate in the open window, as
    /// `(card, effect slot)` — what a `MSG_SELECT_CHAIN` prompt offers besides
    /// "pass". See [`Duel::response_options_for`].
    pub fn chain_response_options(&self) -> Vec<(CardId, usize)> {
        self.response_options_for(self.responder)
    }

    /// The effects `player` may activate in whatever response window is open:
    /// - a **chain** is building → the spell-speed-gated `chainable_effects`;
    /// - no chain but a **timing window** is open (e.g. damage calculation) →
    ///   QUICK effects in their hand whose `event` matches that timing;
    /// - neither → nothing.
    pub fn response_options_for(&self, player: usize) -> Vec<(CardId, usize)> {
        match self.chain.last() {
            Some(top) => self.chainable_effects(player, top),
            None => match self.window_timing {
                Some(timing) => self.timed_hand_quick_effects(player, timing),
                None => Vec::new(),
            },
        }
    }

    /// QUICK effects in `player`'s hand whose `event` matches `timing` and whose
    /// `condition` currently passes — the ones that may start a chain at a timing
    /// window (e.g. Kuriboh at damage calculation). The chain is empty here, so
    /// there's no top link to gate against; QUICK is spell speed 2, enough to open.
    fn timed_hand_quick_effects(&self, player: usize, timing: u32) -> Vec<(CardId, usize)> {
        let hand: Vec<CardId> = {
            let f = self.field.borrow();
            (0..f.hand_count(player))
                .filter_map(|i| f.hand_card(player, i))
                .collect()
        };

        let mut out = Vec::new();
        for card in hand {
            {
                let mut ctx = self.effect_ctx.borrow_mut();
                ctx.activator = player;
                ctx.self_card = Some(card);
            }
            for (slot, effect) in self.effects_of(card).iter().enumerate() {
                if self.effect_kind(effect) == EffectKind::Quick
                    && effect.get::<u32>("event").unwrap_or(0) == timing
                    && self.check_condition(effect).unwrap_or(false)
                {
                    out.push((card, slot));
                }
            }
        }
        out
    }

    pub fn resolve_chain(&mut self) {
        while let Some(link) = self.chain.pop() {
            {
                let mut ctx = self.effect_ctx.borrow_mut();
                ctx.activator = link.activator;
                ctx.targets = link.targets;
                ctx.self_card = Some(link.card);
                ctx.event = link.event; // restore the firing event's details
            }

            let _ = self.resolve_effect(link.effect_seq);

            let is_spell = self
                .effects
                .borrow()
                .get(link.effect_seq)
                .map(|(_, t)| self.effect_kind(t))
                == Some(EffectKind::Activate);
            if is_spell {
                self.send_to(link.card, Zone::GY);
            }
        }
    }
}
