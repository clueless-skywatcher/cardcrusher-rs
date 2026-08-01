-- Effect categories — WHAT an effect does (a bitmask; an effect may do several).
-- Orthogonal to its `kind`; the second arg to add_effect. Mirrors EDOPro's
-- CATEGORY_* set. Add custom ones with the next free bit.
EFF_CAT_DESTROY           = 0x1
EFF_CAT_RELEASE           = 0x2
EFF_CAT_BANISH            = 0x4
EFF_CAT_TO_HAND           = 0x8
EFF_CAT_TO_DECK           = 0x10
EFF_CAT_TO_GRAVE          = 0x20
EFF_CAT_DRAW              = 0x40
EFF_CAT_SEARCH            = 0x80
EFF_CAT_SUMMON            = 0x100
EFF_CAT_SPECIAL_SUMMON    = 0x200
EFF_CAT_TOKEN             = 0x400
EFF_CAT_DAMAGE            = 0x800
EFF_CAT_RECOVER           = 0x1000
EFF_CAT_ATK_CHANGE        = 0x2000
EFF_CAT_DEF_CHANGE        = 0x4000
EFF_CAT_POSITION          = 0x8000
EFF_CAT_CONTROL           = 0x10000
EFF_CAT_NEGATE            = 0x20000
EFF_CAT_COUNTER           = 0x40000
EFF_CAT_EQUIP             = 0x80000
EFF_CAT_FLIP              = 0x100000
EFF_CAT_REVEAL            = 0x200000
EFF_CAT_PRE_DAMAGE_CALC   = 0x400000
