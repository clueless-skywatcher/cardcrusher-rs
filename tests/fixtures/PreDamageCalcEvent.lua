-- PreDamageCalcEvent.lua — rung-4 fixture: a Kuriboh-lite. A monster with a QUICK
-- effect whose timing is damage calculation, activatable straight from the hand.
-- Used to check that the damage-calc window OFFERS a quick-from-hand effect. Its
-- resolve is a no-op — rung 4 only cares that it's offered, not what it does. Fake code.

PreDamageCalcEvent = Card:new(90000011, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 0,
    def = 0,
    level = 1,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Pre Damage Calc Event",
    text = "(test) A Quick effect activatable from the hand at damage calculation.",
})

local e = PreDamageCalcEvent:add_effect(QUICK)
e.event = EVENT_PRE_DAMAGE_CALCULATION -- the timing window it answers

function e:condition(ef)
    return ef:in_hand(YOU)
end

function e:resolve(ef) end
