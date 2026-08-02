-- EventReader.lua — rung-1 fixture for event details. On its own destruction it
-- reads the destroyed card FROM THE EVENT (not from self_card) via
-- get_event_detail, and banishes exactly that card (GY → Banishment). Proves the
-- event's detail bag is queryable by (event_code, key). Fake code (9000001x).

EventReader = Card:new(90000017, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 100,
    def = 100,
    level = 1,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Event Reader",
    text = "(test) When destroyed: banish the card the event reports as destroyed.",
})

local e = EventReader:add_effect(TRIGGER)
e.event = EVENT_DESTROYED

function e:resolve(ef)
    -- The event code guards the query: only returns "card" if the current event is
    -- an EVENT_DESTROYED (else nil).
    local destroyed = ef:get_event_detail(EVENT_DESTROYED, "destroyed_card")
    ef:send({ destroyed }, ZONE_BANISHMENT)
end
