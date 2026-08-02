EVENT_DESTROYED               = 1
EVENT_BATTLE_DESTROYED        = 2
EVENT_BATTLE_STARTED          = 3
EVENT_BATTLE_ENDED            = 4
EVENT_PRE_DAMAGE_CALCULATION  = 5
EVENT_POST_DAMAGE_CALCULATION = 6
EVENT_SPECIAL_SUMMON          = 7
-- The two end-of-turn moments, in the order they fire. END_PHASE_STARTED is for
-- things that happen *during* the End Phase (destroy a borrowed monster);
-- TURN_ENDED is after the End Phase is fully done, and is where "this turn"
-- rules expire. Everything listening to an event fires before anything expiring
-- on it is removed.
EVENT_END_PHASE_STARTED       = 8
EVENT_TURN_ENDED              = 9
