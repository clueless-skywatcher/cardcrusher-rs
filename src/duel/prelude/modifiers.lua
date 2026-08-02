-- Modifier-type codes bridging Rust's `ModifierType` across to Lua (used by
-- add_player_modifier / add_modifier). Values MUST match `ModifierType::from_code`.
MOD_ATK_CHANGE = 0
MOD_DEF_CHANGE = 1
MOD_SET_ATK = 2
MOD_SET_DEF = 3
MOD_NO_BATTLE_DAMAGE = 4

-- Frequency periods for queue(event, {count, period}, fn). Only the count is
-- modeled so far; the period is reserved (per-turn resets etc.).
ONCE = 0
PER_TURN = 1
THIS_TURN = 2
PER_BATTLE = 3
