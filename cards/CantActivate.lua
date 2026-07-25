-- A card whose effect can never be activated: its condition is always false.
-- Used to prove the engine gates activation on `condition`.

CantActivate = Card:new(11111111)

local activate = CantActivate:add_effect(ACTIVATE)

function activate:condition(effect)
    return false
end

function activate:cost(effect)
    effect:pay_lp(500)
end

function activate:resolve(effect)
end
