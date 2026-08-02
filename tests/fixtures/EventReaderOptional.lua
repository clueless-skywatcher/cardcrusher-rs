-- EventReaderOptional.lua — like EventReader but an OPTIONAL trigger (yes/no). On
-- its own destruction, if the player says yes, it reads the destroyed card from the
-- event and banishes it. Proves optional triggers can read event details too.
-- Fake code (9000001x).

EventReaderOptional = Card:new(90000018, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 100,
    def = 100,
    level = 1,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Event Reader Optional",
    text = "(test) When destroyed: you can banish the card the event reports as destroyed.",
})

local e = EventReaderOptional:add_effect(TRIGGER)
e.event = EVENT_DESTROYED
e.optional = true

function e:resolve(ef)
    local destroyed = ef:get_event_detail(EVENT_DESTROYED, "destroyed_card")
    ef:send({ destroyed }, ZONE_BANISHMENT)
end
