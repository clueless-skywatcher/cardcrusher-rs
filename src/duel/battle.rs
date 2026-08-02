//! The Battle Phase: declaring attacks (B2) and resolving battle damage (B3).
//!
//! An attack is declared as `attacker → target`, where a `None` target means a
//! **direct attack** (the opponent controls no monsters). `resolve_battle` then
//! turns that declaration into destruction (via `Duel::destroy`, so the reason is
//! recorded for future triggers) and life-point damage.

use std::cmp::Ordering;

use crate::event::EVENT_PRE_DAMAGE_CALCULATION;
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

    /// The battle damage each player would take from resolving `attacker` vs
    /// `target`, computed WITHOUT applying anything — mirrors `resolve_battle`'s
    /// arithmetic. Returned as `[player0, player1]`. Used to fill `pending_damage`
    /// so a quick effect at the damage-calc window can read it (`e:battle_damage`).
    pub(super) fn battle_damage_preview(
        &self,
        attacker: CardId,
        target: Option<CardId>,
    ) -> [u32; 2] {
        let mut dmg = [0u32; 2];
        let attacker_atk = self.atk_of(attacker).unwrap_or(0);
        let attacker_player = self.controller_of(attacker);

        let Some(target) = target else {
            let defender = (attacker_player + 1) % 2;
            dmg[defender] = attacker_atk.max(0) as u32;
            return dmg;
        };

        let target_player = self.controller_of(target);
        let target_in_attack = self
            .position_of(target)
            .map(|p| p.is_attack())
            .unwrap_or(false);

        if target_in_attack {
            let defender_atk = self.atk_of(target).unwrap_or(0);
            match attacker_atk.cmp(&defender_atk) {
                Ordering::Greater => dmg[target_player] = (attacker_atk - defender_atk) as u32,
                Ordering::Less => dmg[attacker_player] = (defender_atk - attacker_atk) as u32,
                Ordering::Equal => {}
            }
        } else {
            let def = self.def_of(target).unwrap_or(0);
            if attacker_atk < def {
                dmg[attacker_player] = (def - attacker_atk) as u32;
            }
        }
        dmg
    }

    /// Fill `pending_damage` for this battle (so a responder can read
    /// `e:battle_damage()`), then open the before-damage-calculation window.
    pub(super) fn open_before_damage_window(
        &mut self,
        attacker: CardId,
        target: Option<CardId>,
    ) -> bool {
        self.effect_ctx.borrow_mut().pending_damage = self.battle_damage_preview(attacker, target);
        self.open_event_window(EVENT_PRE_DAMAGE_CALCULATION)
    }

    /// Open a response window for a timing event while **no chain** is building
    /// (before/after damage calculation): any player holding a QUICK effect keyed
    /// to `timing` may activate it, the **turn player first** — mirroring EDOPro's
    /// `PointEvent` at `infos.turn_player`. A player with no matching effect is
    /// never prompted (auto-passed), so a window only appears for someone who can
    /// actually act. Returns whether a window opened; if not, `window_timing` is
    /// cleared and the caller proceeds straight to applying the battle.
    pub(super) fn open_event_window(&mut self, timing: u32) -> bool {
        self.window_timing = Some(timing);
        let turn_player = self.turn_hist.last().copied().unwrap_or(0);
        let order = [turn_player, 1 - turn_player];

        // A player is a live responder iff they hold a matching quick effect.
        let mut has_opts = [false; 2];
        for p in order {
            has_opts[p] = !self.response_options_for(p).is_empty();
        }
        let Some(first) = order.into_iter().find(|&p| has_opts[p]) else {
            self.window_timing = None; // nobody can respond → no window
            return false;
        };

        // Seed the pass tracking so a no-option player counts as already passed,
        // then let the first live responder (turn player if they can act) go.
        self.passes = [!has_opts[0], !has_opts[1]];
        self.processor_stack.push(Processor::ChainResponse {
            step: 0,
            player: first,
        });
        true
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
    /// Integer-only; the *controller* of a monster takes its battle damage. This
    /// is the all-in-one form (compute → apply damage → destroy), used when there's
    /// no damage-calc window to interpose. The processor path instead computes the
    /// damage up front (`battle_damage_preview`), lets continuous effects adjust it,
    /// then calls [`apply_battle_damage`] + [`apply_battle_destruction`].
    pub fn resolve_battle(&mut self, attacker: CardId, target: Option<CardId>) {
        let dmg = self.battle_damage_preview(attacker, target);
        self.apply_battle_damage(dmg);
        self.apply_battle_destruction(attacker, target);
    }

    /// Deal already-computed battle damage `[player0, player1]` to the two players.
    /// EDOPro applies damage (BattleCommand step 27) before destruction (step 28).
    pub(super) fn apply_battle_damage(&mut self, dmg: [u32; 2]) {
        use crate::constants::{PLAYER_0, PLAYER_1};
        self.deal_damage(PLAYER_0, dmg[0]);
        self.deal_damage(PLAYER_1, dmg[1]);
    }

    /// Destroy the battle's loser(s) — the destruction half of a resolved attack,
    /// with no life-point effect (that's [`apply_battle_damage`]). A direct attack
    /// destroys nothing.
    pub(super) fn apply_battle_destruction(&mut self, attacker: CardId, target: Option<CardId>) {
        let Some(target) = target else {
            return; // direct attack — nothing to destroy
        };
        let attacker_atk = self.atk_of(attacker).unwrap_or(0);
        let target_in_attack = self
            .position_of(target)
            .map(|p| p.is_attack())
            .unwrap_or(false);

        if target_in_attack {
            let defender_atk = self.atk_of(target).unwrap_or(0);
            match attacker_atk.cmp(&defender_atk) {
                Ordering::Greater => self.destroy(target, REASON_BATTLE),
                Ordering::Less => self.destroy(attacker, REASON_BATTLE),
                Ordering::Equal => {
                    self.destroy(target, REASON_BATTLE);
                    self.destroy(attacker, REASON_BATTLE);
                }
            }
        } else {
            let def = self.def_of(target).unwrap_or(0);
            // ATK beats DEF → destroy the wall; a bigger wall destroys nothing.
            if attacker_atk > def {
                self.destroy(target, REASON_BATTLE);
            }
        }
    }
}
