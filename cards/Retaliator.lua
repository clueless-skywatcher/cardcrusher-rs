-- Retaliator.lua — test fixture for GENERIC event dispatch. Unlike Avenger
-- ("by battle"), this trigger subscribes to the generic EVENT_DESTROYED, so it
-- must fire on ANY destruction (battle or effect). Resolve wipes the opponent's
-- monsters, so a firing is obvious.

Retaliator = Card:new(90000003, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 1000,
    def = 1000,
    level = 4,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Retaliator",
    text = "If this card is destroyed: destroy all monsters your opponent controls.",
})

local t = Retaliator:add_effect(TRIGGER, {EFF_CAT_DESTROY})
t.event = EVENT_DESTROYED -- subscribe to the generic destruction event

function t:resolve(effect)
    effect:destroy(effect:monster_zone(OPPONENT))
end
