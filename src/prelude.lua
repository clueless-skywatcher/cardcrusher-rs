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
    register_effect(effect)
    return effect
end
