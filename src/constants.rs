//! All shared constant values (and the small type aliases they use), in one
//! place: player indices, outbox message codes, and Main-Phase menu commands.

/// The two players, by index — named for readability over bare `0` / `1`.
pub const PLAYER_0: usize = 0;
pub const PLAYER_1: usize = 1;

/// A code written to the outbox saying what happened / what's being asked.
pub type DuelMessage = u8;

pub const MSG_STARTUP: DuelMessage = 1;
pub const MSG_NEW_TURN: DuelMessage = 2;
pub const MSG_SELECT_CARD: DuelMessage = 3;
pub const MSG_SELECT_YESNO: DuelMessage = 4;
pub const MSG_SELECT_CHAIN: DuelMessage = 5;

// Phase-entry messages — one per phase of a turn.
pub const MSG_PHASE_DRAW: DuelMessage = 10;
pub const MSG_PHASE_STANDBY: DuelMessage = 11;
pub const MSG_PHASE_MAIN1: DuelMessage = 12;
pub const MSG_PHASE_BATTLE: DuelMessage = 13;
pub const MSG_PHASE_MAIN2: DuelMessage = 14;
pub const MSG_PHASE_END: DuelMessage = 15;

pub const MSG_SELECT_IDLECMD: DuelMessage = 16;
/// The Battle-Phase menu — "pick a battle command" (attack, or move on).
pub const MSG_SELECT_BATTLECMD: DuelMessage = 17;
/// Choose which opponent monster a declared attack targets (from
/// `attack_targets`) — distinct from `MSG_SELECT_CARD` (effect targeting).
pub const MSG_SELECT_ATTACK_TARGET: DuelMessage = 18;

/// A Main-Phase menu command — the first byte of an idle-command response
/// (`[command, index]`).
pub type Command = u8;

pub const CMD_NEXT_PHASE: Command = 0;
pub const CMD_SUMMON: Command = 1;
pub const CMD_ACTIVATE: Command = 5;
/// Declare an attack — response is `[CMD_ATTACK, attacker_index]`.
pub const CMD_ATTACK: Command = 7;
pub const CMD_SET_SPELL_TRAP: Command = 9;
pub const CMD_PASS: Command = 10;
pub const CMD_RESPONSE: Command = 11;
