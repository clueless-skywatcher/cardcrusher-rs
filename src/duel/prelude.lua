-- The card DSL prelude: the base classes every card builds on.
-- Loaded once into each duel's Lua VM before any card, so `Card`/`Effect` exist.

-- Base class for effects. A card's effect inherits these default stages and
-- overrides only the ones it changes.
Effect = {}
Effect.__index = Effect
function Effect:condition(e) return true end  -- activatable by default
function Effect:cost(e)      end              -- free by default
function Effect:target(e)    end              -- no target by default
function Effect:resolve(e)   end              -- does nothing by default

-- Effect verbs: thin wrappers over the engine's Rust hooks. They read/write the
-- shared effect context, so what a stage does is applied to the real duel.
function Effect:targets()     return effect_targets() end
function Effect:destroy(cards) effect_destroy(cards) end
function Effect:pay_lp(n)      effect_pay_lp(n) end

-- Ask the host to choose `count` cards from `candidates`. This PAUSES the whole
-- duel (coroutine.yield) until the host answers; the engine records the candidate
-- set (so it can reject an empty one and map the picked index back to a card).
-- Because it's plain Lua, the stage suspends linearly.
function Effect:prompt_selection(candidates, count)
    effect_offer(candidates)
    return coroutine.yield(count)
end

-- The monsters a player controls (`who` relative to the activating player).
function Effect:monster_zone(who) return effect_monster_zone(who) end

-- Player references, relative to the activating player.
YOU = 0
OPPONENT = 1

-- Base class for cards.
Card = {}
Card.__index = Card
function Card:new(id)
    return setmetatable({ id = id }, self)
end

-- Make a fresh effect (inheriting Effect's defaults) and hand it to the engine,
-- which remembers it so it can run its stages later.
function Card:add_effect()
    local effect = setmetatable({}, Effect)
    register_effect(self.id, effect)
    return effect
end
