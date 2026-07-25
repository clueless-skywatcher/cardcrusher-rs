//! Loading cards and running their Lua effect stages.
//!
//! An effect stage (`cost`/`resolve`/…) is a Lua method. We call it with the
//! effect table as both `self` and `e`; its verbs record intents into the shared
//! context, and we apply those to the real duel ("describe, then execute").

use mlua::thread::ThreadStatus;

use crate::effect::CostType;
use crate::ids::CardId;
use crate::processor::DuelStatus;
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
        }
    }

    /// Apply a cost that has already been verified payable.
    fn apply_cost(&mut self, cost: &CostType, player: usize) {
        match cost {
            CostType::LifePoints(n) => self.pay_lp(player, *n),
        }
    }

    pub fn resolve_effect(&mut self, effect_idx: usize) -> mlua::Result<()> {
        let effect_table = self.effects.borrow()[effect_idx].1.clone();
        let resolve_func = effect_table.get::<mlua::Function>("resolve")?;
        resolve_func.call::<()>((effect_table.clone(), effect_table))?;

        self.handle_destroys();
        Ok(())
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

        // Set the activator BEFORE `condition` — a condition can ask about
        // YOU/OPPONENT, which are relative to the activating player.
        self.effect_ctx.borrow_mut().activator = player;
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
                self.pending = Some((thread, idx));
                Ok(DuelStatus::Awaiting)
            }
            // No selection needed — pay cost and resolve straight away.
            _ => {
                if !self.pay_cost(idx, player)? {
                    return Ok(DuelStatus::End);
                }
                self.resolve_effect(idx)?;
                Ok(DuelStatus::End)
            }
        }
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
        let (thread, index) = self
            .pending
            .take()
            .expect("nothing is awaiting a selection");
        let chosen = crate::effect::encode_ids(&self.effect_ctx.borrow().targets);
        thread.resume::<mlua::Value>(chosen)?;
        self.resolve_effect(index)?;
        Ok(DuelStatus::End)
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

    fn handle_destroys(&mut self) {
        let to_destroy: Vec<CardId> = self.effect_ctx.borrow_mut().to_destroy.drain(..).collect();
        for card in to_destroy {
            self.send_to(card, Zone::GY);
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
}
