//! The card DSL runtime.
//!
//! **Empty room — built from scratch on Lua (`mlua`).** Built test-first per
//! `PHASE-LUA.md`: a card is a Lua object, its effect stages run on the duel's
//! Lua VM, and `coroutine.yield` pauses an effect mid-run to ask the player
//! something.
//!
//! See `LUA-PRIMER.md` (the language) and `MLUA-GUIDE.md` (the Rust bridge).
