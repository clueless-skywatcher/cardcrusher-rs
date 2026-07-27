PotOfGreed = Card:new(55144522, {
    type = TYPE_SPELL,
    name = "Pot of Greed",
    text = "Draw 2 cards.",
})

local activate = PotOfGreed:add_effect(ACTIVATE, {EFF_CAT_DRAW})

function activate:resolve(effect)
    effect:draw(YOU, 2)
end