//! The Battle Phase: declaring attacks (B2). Damage calculation is B3.
//!
//! An attack is declared as `attacker → target`, where a `None` target means a
//! **direct attack** (the opponent controls no monsters). Declaring only records
//! the attack for now; resolving it into life-point damage comes next.

use crate::ids::CardId;
use crate::processor::Processor;

use super::Duel;

impl Duel {
    /// Queue a stand-alone Battle-Phase menu for player 0 (test entry point).
    pub fn battle_command(&mut self) {
        self.battle_command_for(crate::constants::PLAYER_0);
    }

    /// Queue a stand-alone Battle-Phase menu for a specific player.
    pub fn battle_command_for(&mut self, player: usize) {
        self.processor_stack
            .push(Processor::BattleCommand { step: 0, player });
    }

    /// The monsters `player` can attack with: the ones they control that are
    /// **face-up in attack position** (in id order — `monster_zone` is sorted).
    pub fn attackers(&self, player: usize) -> Vec<CardId> {
        self.monster_zone(player)
            .into_iter()
            .filter(|&c| matches!(self.position_of(c), Some(p) if p.is_attack() && p.is_face_up()))
            .collect()
    }

    /// The monsters an attack by `player` may target: everything the opponent
    /// controls. Empty → the attack can only be direct.
    pub fn attack_targets(&self, player: usize) -> Vec<CardId> {
        let opponent = (player + 1) % 2;
        self.monster_zone(opponent)
    }

    /// Record a declared attack: `attacker` vs `target` (`None` = direct). B3
    /// will resolve this into destruction and life-point damage.
    pub fn declare_attack(&mut self, attacker: CardId, target: Option<CardId>) {
        self.last_attack = Some((attacker, target));
    }

    /// The most recently declared attack, as `(attacker, target)` with `None`
    /// target meaning a direct attack.
    pub fn last_attack(&self) -> Option<(CardId, Option<CardId>)> {
        self.last_attack
    }
}
