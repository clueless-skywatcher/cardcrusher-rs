//! Loading cards and running their Lua effect stages.
//!
//! An effect stage (`cost`/`resolve`/…) is a Lua method. We call it, passing the
//! effect object as `self` and an [`Effect`] (`e`) it records intents into; then
//! we apply those records to the real duel ("describe, then execute").

use crate::effect::Effect;
use crate::ids::CardId;
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
        let effect = self
            .vm
            .create_userdata(Effect::new(self.effect_ctx.clone()))
            .unwrap();
        cost_func
            .call::<()>((effect_table, effect))
            .expect("Cost paid successfully");

        self.handle_lp_payment(player);
    }

    pub fn resolve_effect(&mut self, effect_idx: usize) {
        let effect_table = self.effects.borrow()[effect_idx].clone();
        let resolve_func = effect_table.get::<mlua::Function>("resolve").unwrap();
        let effect = self
            .vm
            .create_userdata(Effect::new(self.effect_ctx.clone()))
            .unwrap();
        resolve_func
            .call::<()>((effect_table, effect))
            .expect("Resolve ran successfully");

        self.handle_destroys();
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
