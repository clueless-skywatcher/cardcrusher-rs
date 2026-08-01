-- Card types (bitmask). The three CLASS bits (MONSTER/SPELL/TRAP) say what a card
-- is; every other bit is a MONSTER subtype. Spell/Trap subtypes live in their own
-- enums (`spell_types.lua` / `trap_types.lua`), so we DIVERGE from EDOPro here
-- (which packs quick-play/continuous/counter/… into this same bitmask).
TYPE_MONSTER    = 0x1
TYPE_SPELL      = 0x2
TYPE_TRAP       = 0x4
TYPE_NORMAL     = 0x10
TYPE_EFFECT     = 0x20
TYPE_FUSION     = 0x40
TYPE_RITUAL     = 0x80
TYPE_SPIRIT     = 0x200
TYPE_UNION      = 0x400
TYPE_GEMINI     = 0x800
TYPE_TUNER      = 0x1000
TYPE_SYNCHRO    = 0x2000
TYPE_TOKEN      = 0x4000
TYPE_FLIP       = 0x200000
TYPE_TOON       = 0x400000
TYPE_XYZ        = 0x800000
TYPE_PENDULUM   = 0x1000000
TYPE_LINK       = 0x4000000
