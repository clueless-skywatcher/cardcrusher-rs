-- SelfDiscard.lua — rung-2 fixture: an effect whose COST is "discard this card".
-- Activating it should send this very card to the GY as a cost. (A monster so the
-- Spell/Trap → S/T-zone lifecycle doesn't interfere.) Fake code (9000001x).

SelfDiscard = Card:new(90000012, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 100,
    def = 100,
    level = 1,
    attribute = ATTRIBUTE_EARTH,
    race = RACE_WARRIOR,
    name = "Self Discard",
    text = "(test) Its activation cost is discarding this card.",
})

local e = SelfDiscard:add_effect(IGNITION)

function e:cost(ef)
    ef:discard_self() -- send self (hand -> GY) as a cost
end
