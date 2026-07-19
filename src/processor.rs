//! The engine's heartbeat: a pausable to-do stack.
//!
//! **Milestone 1:** empty room. The real machine arrives in Milestone 3.
//!
//! **The mental model:** the game is a stack of sticky notes. Each note is a job
//! in progress ("it's someone's turn", "we're summoning", "waiting for a pick"),
//! and each note remembers *how far along it is*.
//!
//! The whole engine is one tiny loop that does this forever:
//!
//! ```text
//! loop {
//!     look at the TOP note
//!     do ONE small step of it
//!     then either:
//!         ✅ finish it  → throw the note away
//!         ⏸  pause it   → leave the note, bump its step number
//! }
//! ```
//!
//! **Why a stack of steps instead of normal function calls?** Because the engine
//! must be able to *freeze* mid-job (to ask a human "which card?") and thaw later
//! exactly where it left off. A paused note on a stack can do that; a half-finished
//! normal function call cannot.

pub type Step = u16;
pub type DuelMessage = u8;

#[derive(Debug)]
pub enum Processor {
    Startup { step: Step },
    Turn { step: Step, player: usize },
    SelectCard { step: Step },
    IdleCommand { step: Step, player: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub enum DuelStatus {
    Continue,
    Awaiting,
    End,
}

/// Outbox message codes (stubs for now).
pub const MSG_STARTUP: DuelMessage = 1;
pub const MSG_NEW_TURN: DuelMessage = 2;
pub const MSG_SELECT_CARD: DuelMessage = 3;

/// Phase-entry message codes — one per phase of a turn.
pub const MSG_PHASE_DRAW: DuelMessage = 10;
pub const MSG_PHASE_STANDBY: DuelMessage = 11;
pub const MSG_PHASE_MAIN1: DuelMessage = 12;
pub const MSG_PHASE_BATTLE: DuelMessage = 13;
pub const MSG_PHASE_MAIN2: DuelMessage = 14;
pub const MSG_PHASE_END: DuelMessage = 15;

pub const MSG_SELECT_IDLECMD: DuelMessage = 16;

impl Processor {
    /// Does pausing on this task mean we must stop and ask a human?
    pub fn needs_answer(&self) -> bool {
        match self {
            Processor::Startup { .. } | Processor::Turn { .. } => false,
            Processor::SelectCard { .. } => true,
            Processor::IdleCommand { .. } => true,
        }
    }
}
