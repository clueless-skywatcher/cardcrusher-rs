-- OptionalAvenger.lua — test fixture for OPTIONAL triggers ("you can"). Same
-- board-wipe as Avenger, but its controller may decline. `e.optional = true`
-- marks it TRIGGER_O (vs the default, mandatory TRIGGER_F).

OptionalAvenger = Card:new(90000005, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 1000,
    def = 1000,
    level = 4,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Optional Avenger",
    text = "If this card is destroyed: you can destroy all monsters your opponent controls.",
})

local e = OptionalAvenger:add_effect(TRIGGER, { EFF_CAT_DESTROY })
e.event = EVENT_DESTROYED
e.optional = true

function e:resolve(effect)
    effect:destroy(effect:monster_zone(OPPONENT))
end
