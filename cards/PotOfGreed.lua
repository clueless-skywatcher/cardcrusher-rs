PotOfGreed = Card:new(55144522)

local activate = PotOfGreed:add_effect(ACTIVATE)

function activate:resolve(effect)
    effect:draw(YOU, 2)
end