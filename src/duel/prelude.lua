-- The card DSL prelude: the base classes every card builds on.
-- Loaded once into each duel's Lua VM before any card, so `Card`/`Effect` exist.

-- Base class for effects. A card's effect inherits these default stages and
-- overrides only the ones it changes.
Effect = {}
Effect.__index = Effect
function Effect:condition(e) return true end -- activatable by default

function Effect:cost(e) end                  -- free by default

function Effect:target(e) end                -- no target by default

function Effect:resolve(e) end               -- does nothing by default

-- Effect verbs: thin wrappers over the engine's Rust hooks. They read/write the
-- shared effect context, so what a stage does is applied to the real duel.
function Effect:targets() return effect_targets() end

function Effect:destroy(cards) effect_destroy(cards) end

function Effect:pay_lp(n) effect_pay_lp(n) end

-- Ask the host to choose `count` cards from `candidates`. This PAUSES the whole
-- duel (coroutine.yield) until the host answers; the engine records the candidate
-- set (so it can reject an empty one and map the picked index back to a card).
-- Because it's plain Lua, the stage suspends linearly.
function Effect:prompt_selection(candidates, count)
    effect_prompt_selection(candidates)
    return coroutine.yield(count)
end

-- The monsters a player controls (`who` relative to the activating player).
function Effect:monster_zone(who) return effect_monster_zone(who) end

-- Player references, relative to the activating player.
YOU          = 0
OPPONENT     = 1

-- Effect kinds — how/where an effect is activated (passed to add_effect).
ACTIVATE     = 0 -- a Spell/Trap card's activation (from hand / set zone)
IGNITION     = 1 -- a manual effect on a card you control, on your Main Phase
QUICK        = 2 -- quick effect (needs the chain engine — not activatable yet)
TRIGGER      = 3 -- fires on an event (needs the event engine — not activatable yet)

-- Card types (bitmask) — mirror EDOPro's TYPE_* (ocgcore/common.h).
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
TYPE_QUICKPLAY  = 0x10000
TYPE_CONTINUOUS = 0x20000
TYPE_EQUIP      = 0x40000
TYPE_FIELD      = 0x80000
TYPE_COUNTER    = 0x100000
TYPE_FLIP       = 0x200000
TYPE_TOON       = 0x400000
TYPE_XYZ        = 0x800000
TYPE_PENDULUM   = 0x1000000
TYPE_LINK       = 0x4000000

-- Attributes (bitmask) — a monster has exactly one.
ATTRIBUTE_EARTH  = 0x01
ATTRIBUTE_WATER  = 0x02
ATTRIBUTE_FIRE   = 0x04
ATTRIBUTE_WIND   = 0x08
ATTRIBUTE_LIGHT  = 0x10
ATTRIBUTE_DARK   = 0x20
ATTRIBUTE_DIVINE = 0x40

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
-- never hard-codes the names — so just add your own here with the next free bit:
--   TYPE_RUSH  = 0x8000000   -- a custom mechanic flag
--   RACE_SLIME = 0x100000000 -- a custom monster type

-- Base class for cards.
Card         = {}
Card.__index = Card
-- `data` (optional) is the card's printed record: { type, atk, def, level,
-- attribute, race, text }. The engine harvests it (register_card) keyed by id.
function Card:new(id, data)
    register_card(id, data or {})
    return setmetatable({ id = id }, self)
end

-- Make a fresh effect (inheriting Effect's defaults) and hand it to the engine,
-- which remembers it so it can run its stages later.
function Card:add_effect(kind)
    local effect = setmetatable({ kind = kind }, Effect)
    register_effect(self.id, effect)
    return effect
end
