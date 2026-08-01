-- The card DSL base classes (Effect, Card) and the effect verbs. Loaded after the
-- constant tables (players / effect_kinds / categories / card_types / attributes /
-- races), before any card — so those names exist when a card is loaded.

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

-- Move card(s) to a zone (ZONE_HAND / ZONE_DECK / ZONE_GY / ZONE_BANISHMENT / …).
-- A plain relocation — NOT a destruction (no "destroyed" trigger fires).
function Effect:send(cards, zone) effect_send(cards, zone) end

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

-- Base class for cards.
Card         = {}
Card.__index = Card
-- `data` (optional) is the card's printed record: { type, atk, def, level,
-- attribute, race, name, text }. The engine harvests it (register_card) by id.
function Card:new(id, data)
    register_card(id, data or {})
    return setmetatable({ id = id }, self)
end

-- Make a fresh effect (inheriting Effect's defaults) and hand it to the engine.
--   kind     — ACTIVATE / IGNITION / QUICK / TRIGGER (how/where it acts).
--   category — an EFF_CAT_* bitmask of WHAT it does (optional; 0 if omitted).
-- Both ride on the effect table for the engine to read.
function Card:add_effect(kind, category)
    local effect = setmetatable({ kind = kind, category = category or {} }, Effect)
    register_effect(self.id, effect)
    return effect
end
