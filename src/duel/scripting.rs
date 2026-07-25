//! Loading cards and running their Lua effect stages.
//!
//! An effect stage (`cost`/`resolve`/…) is a Lua method. We call it with the
//! effect table as both `self` and `e`; its verbs record intents into the shared
//! context, and we apply those to the real duel ("describe, then execute").

use mlua::thread::ThreadStatus;

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

    pub fn pay_cost(&mut self, effect_idx: usize, player: usize) -> mlua::Result<()> {
        let effect_table = self.effects.borrow()[effect_idx].clone();
        let cost_func = effect_table.get::<mlua::Function>("cost")?;
        // The effect table is both `self` and `e` — its verbs reach the context.
        cost_func.call::<()>((effect_table.clone(), effect_table))?;

        self.handle_lp_payment(player);
        Ok(())
    }

    pub fn resolve_effect(&mut self, effect_idx: usize) -> mlua::Result<()> {
        let effect_table = self.effects.borrow()[effect_idx].clone();
        let resolve_func = effect_table.get::<mlua::Function>("resolve")?;
        resolve_func.call::<()>((effect_table.clone(), effect_table))?;

        self.handle_destroys();
        Ok(())
    }

    // ===== M4/M5: the coroutine bridge ======================================

    /// Activate an effect. Run its `target` stage on a Lua coroutine FIRST (so we
    /// learn the candidate set), then:
    /// - it asked for a selection with legal candidates → pay cost, freeze
    ///   (`Awaiting`) for the pick;
    /// - it asked but the candidate set is empty → reject up front (no cost);
    /// - it asked for nothing → pay cost and resolve immediately.
    ///
    /// Cost is paid only once the activation is committed — never on a rejection.
    pub fn activate(&mut self, effect_idx: usize, player: usize) -> mlua::Result<DuelStatus> {
        self.effect_ctx.borrow_mut().activator = player;

        let effect = self.effects.borrow()[effect_idx].clone();
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
                self.pay_cost(effect_idx, player)?;
                self.pending = Some((thread, effect_idx));
                Ok(DuelStatus::Awaiting)
            }
            // No selection needed — pay cost and resolve straight away.
            _ => {
                self.pay_cost(effect_idx, player)?;
                self.resolve_effect(effect_idx)?;
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

    fn handle_destroys(&mut self) {
        let to_destroy: Vec<CardId> = self.effect_ctx.borrow_mut().to_destroy.drain(..).collect();
        for card in to_destroy {
            self.send_to(card, Zone::GY);
        }
    }

    fn handle_lp_payment(&mut self, player: usize) {
        let lp_to_pay: u32 = std::mem::take(&mut self.effect_ctx.borrow_mut().lp_payment);
        self.pay_lp(player, lp_to_pay);
    }
}
