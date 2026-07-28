-- Avenger.lua — a TRIGGER-effect monster used to exercise the event engine.
-- "When this card is destroyed by battle: destroy all monsters your opponent
--  controls." The trigger fires off the destruction *event*, not an activation.

Avenger = Card:new(90000001, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 1000,
    def = 1000,
    level = 4,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Avenger",
    text = "If this card is destroyed by battle: destroy all monsters your opponent controls.",
})

local avenge = Avenger:add_effect(TRIGGER, {EFF_CAT_DESTROY})
avenge.event = EVENT_BATTLE_DESTROYED

function avenge:resolve(effect)
    -- OPPONENT is relative to this card's controller.
    effect:destroy(effect:monster_zone(OPPONENT))
end
