//! The board & game state: the card arena, deck/hand piles, zones, movement,
//! life points, and win conditions.

use crate::card::{Card, CardData};
use crate::constants::{PLAYER_0, PLAYER_1};
use crate::event::{DuelEvent, EVENT_BATTLE_DESTROYED, EVENT_DESTROYED};
use crate::ids::CardId;
use crate::modifiers::ModifierType::{AtkChange, DefChange, SetAtk, SetDef};
use crate::modifiers::{Modifier, ModifierType};
use crate::position::Position;
use crate::reason::{Reason, REASON_BATTLE, REASON_DESTROY};
use crate::zone::Zone;

use super::{Duel, WinReason, Winner};

impl Duel {
    // ===== Card arena =======================================================

    pub fn add_card(&mut self, card: Card) -> CardId {
        self.cards.insert(card)
    }

    /// Make a card of `code` with the stats its loaded script declared. If the
    /// code was never loaded, the stats are all zero (a bare card).
    pub fn make_card(&self, code: u32) -> Card {
        let data = self
            .card_data
            .borrow()
            .get(&code)
            .cloned()
            .unwrap_or_default();
        Card::with_data(code, data)
    }

    // ===== Card stats (harvested from the script) ===========================

    /// The full printed record of a card instance (type/atk/def/level/…).
    pub fn card_data(&self, card: CardId) -> Option<&CardData> {
        self.get_card(card).map(|c| &c.data)
    }

    /// A monster's ATK — its printed value with every ATK modifier folded in, by
    /// priority (`SetAtk` before `AtkChange`; ties in insertion order), floored at
    /// 0. `SetAtk` establishes the value, `AtkChange` stacks on top.
    pub fn atk_of(&self, card: CardId) -> Option<i32> {
        let card = self.cards.get(card)?;
        // Gather ATK modifiers and stable-sort by priority (insertion breaks ties).
        let mut mods: Vec<ModifierType> = card
            .modifiers
            .iter()
            .map(|m| m.mod_type)
            .filter(|mt| matches!(mt, AtkChange(_) | SetAtk(_)))
            .collect();
        mods.sort_by_key(|mt| mt.priority());

        let mut atk = card.data.atk;
        for mt in mods {
            match mt {
                SetAtk(n) => atk = n,
                AtkChange(n) => atk += n,
                _ => {}
            }
        }
        Some(atk.max(0))
    }

    /// A monster's DEF — printed value with every DEF modifier folded in by
    /// priority (`SetDef` before `DefChange`; ties in insertion order), floored at
    /// 0. Symmetric with [`atk_of`].
    pub fn def_of(&self, card: CardId) -> Option<i32> {
        let card = self.cards.get(card)?;
        let mut mods: Vec<ModifierType> = card
            .modifiers
            .iter()
            .map(|m| m.mod_type)
            .filter(|mt| matches!(mt, DefChange(_) | SetDef(_)))
            .collect();
        mods.sort_by_key(|mt| mt.priority());

        let mut def = card.data.def;
        for mt in mods {
            match mt {
                SetDef(n) => def = n,
                DefChange(n) => def += n,
                _ => {}
            }
        }
        Some(def.max(0))
    }

    /// A monster's level (for tribute/level checks later). `None` if the card
    /// doesn't exist *or* has no level (Spells/Traps).
    pub fn level_of(&self, card: CardId) -> Option<u32> {
        self.get_card(card).and_then(|c| c.data.level)
    }

    pub fn get_card(&self, id: CardId) -> Option<&Card> {
        self.cards.get(id)
    }

    pub fn remove_card(&mut self, id: CardId) -> Option<Card> {
        self.cards.remove(id)
    }

    // ===== Deck & hand ======================================================

    /// Create a card and put it on the bottom of a player's deck. The card starts
    /// out **owned and controlled** by that player.
    pub fn add_to_deck(&mut self, player: usize, mut card: Card) -> CardId {
        card.owner = player;
        let id = self.cards.insert(card);
        self.field.borrow_mut().add_to_deck(player, id);
        id
    }

    /// Create a card and put it into a player's hand. The card starts out **owned
    /// and controlled** by that player.
    pub fn add_to_hand(&mut self, player: usize, mut card: Card) -> CardId {
        card.owner = player;
        let id = self.cards.insert(card);
        self.field.borrow_mut().add_to_hand(player, id);
        id
    }

    /// Draw `count` cards off the top of a player's deck into their hand. If the
    /// deck can't supply them all, that player decks out (a loss).
    pub fn draw(&mut self, player: usize, count: usize) -> Vec<CardId> {
        let drawn = self.field.borrow_mut().draw(player, count);
        if drawn.len() < count {
            self.decked_out[player] = true;
        }
        self.check_win();
        drawn
    }

    pub fn deck_count(&self, player: usize) -> usize {
        self.field.borrow().deck_count(player)
    }

    pub fn hand_count(&self, player: usize) -> usize {
        self.field.borrow().hand_count(player)
    }

    /// The card at a given slot in a player's hand, if any.
    pub fn hand_card(&self, player: usize, index: usize) -> Option<CardId> {
        self.field.borrow().hand_card(player, index)
    }

    /// The cards a player controls in their Monster Zone.
    pub fn monster_zone(&self, player: usize) -> Vec<CardId> {
        self.field.borrow().monster_zone(player)
    }

    /// The cards a player controls in their Spell & Trap Zone.
    pub fn spell_trap_zone(&self, player: usize) -> Vec<CardId> {
        self.field.borrow().cards_in(player, Zone::SpellTrapZone)
    }

    /// The cards in a player's Graveyard.
    pub fn graveyard(&self, player: usize) -> Vec<CardId> {
        self.field.borrow().cards_in(player, Zone::GY)
    }

    // ===== Zones & movement =================================================

    pub fn place(&mut self, player: usize, card: CardId, zone: Zone) {
        self.field.borrow_mut().place(player, card, zone);
    }

    pub fn zone_of(&self, card: CardId) -> Option<Zone> {
        self.field.borrow().zone_of(card)
    }

    /// Who currently **controls** `card` — the player whose zone it sits in
    /// (defaults to player 0 if unplaced). May differ from its owner.
    pub fn controller_of(&self, card: CardId) -> usize {
        self.field.borrow().controller_of(card).unwrap_or(0)
    }

    /// Who **owns** `card` — whose deck it belongs to. Fixed for the card's life;
    /// unlike control, ownership never changes.
    pub fn owner_of(&self, card: CardId) -> usize {
        self.get_card(card).map(|c| c.owner).unwrap_or(0)
    }

    pub fn send_to(&mut self, card: CardId, zone: Zone) {
        self.field.borrow_mut().send_to(card, zone);
    }

    /// **Destroy** `card` for `reason` (e.g. `REASON_BATTLE`), sending it to the
    /// GY. Unlike a bare `send_to`, this is the single chokepoint for destruction:
    /// it records *why* on the card (OR-ing in `REASON_DESTROY`) so that "when
    /// destroyed by battle/effect" triggers can fire here once the event engine
    /// exists. (A card merely *sent* to the GY — discarded, tributed — is not
    /// "destroyed", which is why the two paths are separate.)
    pub fn destroy(&mut self, card: CardId, reason: Reason) {
        if let Some(c) = self.cards.get_mut(card) {
            c.reason = REASON_DESTROY | reason;
        }
        self.send_to(card, Zone::GY);
        self.events.push_back(DuelEvent {
            code: EVENT_DESTROYED,
            card,
            reason: REASON_DESTROY | reason,
        });
        if reason & REASON_BATTLE != 0 {
            self.events.push_back(DuelEvent {
                code: EVENT_BATTLE_DESTROYED,
                card,
                reason: REASON_DESTROY | reason,
            });
        }
    }

    /// Put a card onto the field as a monster, **face-up in attack position**. A
    /// shared primitive — the menu (Normal Summon) and card effects (Special
    /// Summon) both call it, from any source zone; the caller decides what's legal
    /// (e.g. the once-per-turn Normal Summon limit lives in the menu, not here).
    pub fn summon(&mut self, card: CardId) {
        self.field.borrow_mut().send_to(card, Zone::MonsterZone);
        self.set_position(card, Position::FaceUpAttack);
    }

    /// Set a monster **face-down in defense position** (the monster equivalent of
    /// "Set"). Like `summon`, a primitive — the caller enforces legality.
    pub fn set_monster(&mut self, card: CardId) {
        self.field.borrow_mut().send_to(card, Zone::MonsterZone);
        self.set_position(card, Position::FaceDownDefense);
    }

    /// A monster's current battle position — `None` unless it's in a Monster Zone
    /// (a card in hand / GY / deck has no battle position).
    pub fn position_of(&self, card: CardId) -> Option<Position> {
        match self.zone_of(card) {
            Some(Zone::MonsterZone) => self.get_card(card).map(|c| c.position),
            _ => None,
        }
    }

    /// Change a monster's position — flip it face-up, or switch attack/defense.
    /// A no-op unless the card is in a Monster Zone.
    pub fn change_position(&mut self, card: CardId, pos: Position) {
        if self.zone_of(card) == Some(Zone::MonsterZone) {
            self.set_position(card, pos);
        }
    }

    /// Stamp a position onto the card instance (unconditional; callers gate it).
    fn set_position(&mut self, card: CardId, pos: Position) {
        if let Some(c) = self.cards.get_mut(card) {
            c.position = pos;
        }
    }

    /// Whether `player` may still Normal Summon (or Set) this turn. Base rule:
    /// **once per turn** — the counter resets at the start of each turn. (Special
    /// Summons don't go through this, so they aren't limited by it.)
    pub fn can_normal_summon(&self, player: usize) -> bool {
        self.normal_summons[player] < 1
    }

    /// Record that `player` used their Normal Summon this turn.
    pub fn record_normal_summon(&mut self, player: usize) {
        self.normal_summons[player] += 1;
    }

    /// Reset the Normal-Summon counter (called at the start of each turn).
    pub(crate) fn reset_normal_summons(&mut self) {
        self.normal_summons = [0, 0];
    }

    /// Clear the per-turn attack record (called at the start of each turn).
    pub(crate) fn reset_attacks(&mut self) {
        self.attacked_this_turn.clear();
    }

    /// Set a card face-down in the spell/trap zone. Shared by the menu and card
    /// effects; works regardless of the card's source zone.
    pub fn set_spell_trap(&mut self, card: CardId) {
        self.field.borrow_mut().send_to(card, Zone::SpellTrapZone);
    }

    // ===== Life points & win conditions =====================================

    pub fn life_points(&self, player: usize) -> u32 {
        self.lps[player]
    }

    pub fn pay_lp(&mut self, player: usize, lp: u32) {
        self.lps[player] = self.lps[player].saturating_sub(lp);
        self.check_win();
    }

    pub fn deal_damage(&mut self, player: usize, lp: u32) {
        self.lps[player] = self.lps[player].saturating_sub(lp);
        self.check_win();
    }

    pub fn result(&self) -> Option<Winner> {
        self.result
    }

    pub fn win_reason(&self) -> Option<WinReason> {
        self.win_reason
    }

    /// Re-evaluate the win conditions from scratch: a player at 0 LP or decked
    /// out has lost. Seeing BOTH players lets us tell a single loss from a
    /// simultaneous draw.
    fn check_win(&mut self) {
        let p0_lost = self.lps[PLAYER_0] == 0 || self.decked_out[PLAYER_0];
        let p1_lost = self.lps[PLAYER_1] == 0 || self.decked_out[PLAYER_1];

        // A loser's reason: LP if their life is gone, otherwise deck-out.
        let p0_reason = if self.lps[PLAYER_0] == 0 {
            WinReason::LifePointsDepleted
        } else {
            WinReason::DeckOut
        };
        let p1_reason = if self.lps[PLAYER_1] == 0 {
            WinReason::LifePointsDepleted
        } else {
            WinReason::DeckOut
        };

        match (p0_lost, p1_lost) {
            (true, true) => {
                self.result = Some(Winner::Draw);
                self.win_reason = Some(p0_reason);
            }
            (true, false) => {
                self.result = Some(Winner::Player(PLAYER_1));
                self.win_reason = Some(p0_reason);
            }
            (false, true) => {
                self.result = Some(Winner::Player(PLAYER_0));
                self.win_reason = Some(p1_reason);
            }
            // Nobody is currently losing — leave any decided result untouched. A
            // win is sticky; we never "un-win" (e.g. a future heal above 0 LP).
            (false, false) => {}
        }
    }

    /// Hand out the next unique modifier id (counter lives in `ctx`, so a Lua verb
    /// and the engine draw from the same sequence).
    fn next_modifier_id(&self) -> u32 {
        let mut ctx = self.effect_ctx.borrow_mut();
        ctx.next_modifier_id += 1;
        ctx.next_modifier_id
    }

    /// Add a modifier to `card`, tagged with the `source` card that produced it.
    /// Returns the new modifier's id (for later `remove_modifier`).
    pub fn add_modifier(&mut self, card_id: CardId, source: CardId, mod_type: ModifierType) -> u32 {
        let id = self.next_modifier_id();
        if let Some(card) = self.cards.get_mut(card_id) {
            card.modifiers.push(Modifier {
                id,
                source,
                mod_type,
            });
        }
        id
    }

    /// Remove every modifier produced by `source` — from every card it touched
    /// AND from both players' modifier lists — e.g. when the source's continuous
    /// effect stops applying.
    pub fn remove_modifiers_from(&mut self, source: CardId) {
        for (_, card) in self.cards.iter_mut() {
            card.modifiers.retain(|m| m.source != source);
        }
        for mods in self.player_modifiers.iter_mut() {
            mods.retain(|m| m.source != source);
        }
    }

    /// Remove the single modifier with `id` — from wherever it lives (any card or
    /// either player's list).
    pub fn remove_modifier(&mut self, id: u32) {
        for (_, card) in self.cards.iter_mut() {
            card.modifiers.retain(|m| m.id != id);
        }
        for mods in self.player_modifiers.iter_mut() {
            mods.retain(|m| m.id != id);
        }
    }

    /// Add a modifier to `player` (not a card), tagged with its `source`. Returns
    /// the new modifier's id.
    pub fn add_player_modifier(
        &mut self,
        player: usize,
        source: CardId,
        mod_type: ModifierType,
    ) -> u32 {
        let id = self.next_modifier_id();
        self.player_modifiers[player].push(Modifier {
            id,
            source,
            mod_type,
        });
        id
    }

    /// Whether `player` can currently take battle damage — false while any
    /// `NoBattleDamage` modifier is in force for them (e.g. Kuriboh).
    pub fn can_take_battle_damage(&self, player: usize) -> bool {
        !self.player_modifiers[player]
            .iter()
            .any(|m| m.mod_type == ModifierType::NoBattleDamage)
    }
}
