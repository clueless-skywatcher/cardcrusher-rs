//! Loading cards and running their Lua effect stages.
//!
//! An effect stage (`cost`/`resolve`/…) is a Lua method. We call it with the
//! effect table as both `self` and `e`; its verbs record intents into the shared
//! context, and we apply those to the real duel ("describe, then execute").

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

    pub fn pay_cost(&mut self, effect_idx: usize, player: usize) {
        let effect_table = self.effects.borrow()[effect_idx].clone();
        let cost_func = effect_table.get::<mlua::Function>("cost").unwrap();
        // The effect table is both `self` and `e` — its verbs reach the context.
        cost_func
            .call::<()>((effect_table.clone(), effect_table))
            .expect("Cost paid successfully");

        self.handle_lp_payment(player);
    }

    pub fn resolve_effect(&mut self, effect_idx: usize) {
        let effect_table = self.effects.borrow()[effect_idx].clone();
        let resolve_func = effect_table.get::<mlua::Function>("resolve").unwrap();
        resolve_func
            .call::<()>((effect_table.clone(), effect_table))
            .expect("Resolve ran successfully");

        self.handle_destroys();
    }

    // ===== M4: the coroutine bridge (STUBS — see the PR for how to fill these) =====

    /// Activate an effect: pay its cost, then run its `target` stage on a Lua
    /// coroutine. If the stage yields for a selection, freeze (`Awaiting`);
    /// otherwise resolve immediately.
    pub fn activate(&mut self, _effect_idx: usize, _player: usize) -> DuelStatus {
        todo!("M4: run `target` on a coroutine thread; freeze if it yields")
    }

    /// Supply the player's chosen cards as the resolving effect's targets.
    pub fn answer_selection(&mut self, _cards: Vec<CardId>) {
        todo!("M4: record the chosen cards as targets")
    }

    /// Resume the frozen effect with the answer, then resolve it.
    pub fn resume(&mut self) -> DuelStatus {
        todo!("M4: resume the paused thread, then resolve the effect")
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
