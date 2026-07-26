//! The Battle Phase: declaring attacks (B2) and resolving battle damage (B3).
//!
//! An attack is declared as `attacker → target`, where a `None` target means a
//! **direct attack** (the opponent controls no monsters). `resolve_battle` then
//! turns that declaration into destruction (via `Duel::destroy`, so the reason is
//! recorded for future triggers) and life-point damage.

use std::cmp::Ordering;

use crate::ids::CardId;
use crate::processor::Processor;
use crate::reason::REASON_BATTLE;

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
            .filter(|&c| !self.attacked_this_turn.contains(&c))
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
        self.attacked_this_turn.insert(attacker);
    }

    /// The most recently declared attack, as `(attacker, target)` with `None`
    /// target meaning a direct attack.
    pub fn last_attack(&self) -> Option<(CardId, Option<CardId>)> {
        self.last_attack
    }

    /// Resolve a declared attack into destruction and life-point damage.
    ///
    /// - **Direct** (`target` is `None`): the attacker's ATK hits the opponent.
    /// - **Vs an attack-position monster** (ATK vs ATK): higher wins → loser to
    ///   the GY; the loser's controller takes the difference. A tie destroys both.
    /// - **Vs a defense-position monster** (ATK vs DEF): if ATK > DEF the target
    ///   is destroyed (no damage); if ATK < DEF nothing is destroyed but the
    ///   attacker's controller takes the difference; equal does nothing.
    ///
    /// Integer-only; the *controller* of a monster takes its battle damage.
    pub fn resolve_battle(&mut self, attacker: CardId, target: Option<CardId>) {
        let attacker_atk = self.atk_of(attacker).unwrap_or(0);
        let attacker_player = self.controller_of(attacker);

        // Direct attack: full ATK straight to the defending player.
        let Some(target) = target else {
            let defender = (attacker_player + 1) % 2;
            self.deal_damage(defender, attacker_atk.max(0) as u32);
            return;
        };

        let target_player = self.controller_of(target);
        let target_in_attack = self
            .position_of(target)
            .map(|p| p.is_attack())
            .unwrap_or(false);

        if target_in_attack {
            let defender_atk = self.atk_of(target).unwrap_or(0);
            match attacker_atk.cmp(&defender_atk) {
                Ordering::Greater => {
                    self.destroy(target, REASON_BATTLE);
                    self.deal_damage(target_player, (attacker_atk - defender_atk) as u32);
                }
                Ordering::Less => {
                    self.destroy(attacker, REASON_BATTLE);
                    self.deal_damage(attacker_player, (defender_atk - attacker_atk) as u32);
                }
                Ordering::Equal => {
                    self.destroy(target, REASON_BATTLE);
                    self.destroy(attacker, REASON_BATTLE);
                }
            }
        } else {
            let def = self.def_of(target).unwrap_or(0);
            match attacker_atk.cmp(&def) {
                // ATK beats DEF → destroy the wall, but no damage seeps through.
                std::cmp::Ordering::Greater => self.destroy(target, REASON_BATTLE),
                // Bounced off a bigger wall → the attacker's side takes the hit.
                std::cmp::Ordering::Less => {
                    self.deal_damage(attacker_player, (def - attacker_atk) as u32)
                }
                std::cmp::Ordering::Equal => {}
            }
        }
    }
}
