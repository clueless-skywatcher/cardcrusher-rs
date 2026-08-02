-- HandOnly.lua — rung-1 fixture for self-card tracking (the Kuriboh line). Its
-- only condition is `in_hand()`, so it can be activated from the hand but not from
-- the field — which requires the engine to track WHICH card instance is activating
-- (`EffectContext.self_card`). Fake code (9000001x).

HandOnly = Card:new(90000010, {
    type = TYPE_SPELL,
    spell_type = SPELL_NORMAL,
    name = "Hand Only",
    text = "(test) Activatable only while this card is in the hand.",
})

local e = HandOnly:add_effect(ACTIVATE)

function e:condition(ef)
    return ef:in_hand(YOU)
end
