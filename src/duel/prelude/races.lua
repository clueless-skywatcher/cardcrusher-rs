-- Monster Types ("race" in EDOPro; bitmask).
RACE_WARRIOR      = 0x1
RACE_SPELLCASTER  = 0x2
RACE_FAIRY        = 0x4
RACE_FIEND        = 0x8
RACE_ZOMBIE       = 0x10
RACE_MACHINE      = 0x20
RACE_AQUA         = 0x40
RACE_PYRO         = 0x80
RACE_ROCK         = 0x100
RACE_WINGEDBEAST  = 0x200
RACE_PLANT        = 0x400
RACE_INSECT       = 0x800
RACE_THUNDER      = 0x1000
RACE_DRAGON       = 0x2000
RACE_BEAST        = 0x4000
RACE_BEASTWARRIOR = 0x8000
RACE_DINOSAUR     = 0x10000
RACE_FISH         = 0x20000
RACE_SEASERPENT   = 0x40000
RACE_REPTILE      = 0x80000
RACE_PSYCHIC      = 0x100000
RACE_DIVINE       = 0x200000
RACE_CREATORGOD   = 0x400000
RACE_WYRM         = 0x800000
RACE_CYBERSE      = 0x1000000
RACE_ILLUSION     = 0x2000000
RACE_CYBORG       = 0x4000000
RACE_MAGICALKNIGHT    = 0x8000000
RACE_HIGHDRAGON       = 0x10000000
RACE_OMEGAPSYCHIC     = 0x20000000
RACE_CELESTIALWARRIOR = 0x40000000
RACE_GALAXY           = 0x80000000

-- Custom types/races/attributes: the engine stores these as opaque bitmasks and
-- never hard-codes the names — so just add your own with the next free bit, e.g.
--   RACE_SLIME = 0x100000000
