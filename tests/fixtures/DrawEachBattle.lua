-- DrawEachBattle.lua — a tracked mini-Maxx "C". Same shape as the real card, but
-- hung on a battle event the engine already raises, so the repeating draws are easy
-- to drive and count in a test.
--   Maxx "C"        — each time the OPPONENT Special Summons → draw 1, this turn
--   DrawEachBattle  — each time a battle ENDS → draw 1, this turn
-- Exercises: a rule that fires MORE THAN ONCE, the closure receiving the event, the
-- draw verb landing cards in hand, and the rule expiring at the end of the turn.
-- Free chain like the real card, so there's no `.event` — the quick-from-hand
-- window path is already covered by HandNegate.
-- Fake code (9000001x).

DrawEachBattle = Card:new(90000019, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 100,
    def = 100,
    level = 1,
    attribute = ATTRIBUTE_EARTH,
    race = RACE_INSECT,
    name = "Draw Each Battle",
    text =
    "(test) During your opponent's turn (Quick Effect): You can send this card from your hand to the GY; this turn, each time a battle ends, immediately draw 1 card.",
})

local drawer = DrawEachBattle:add_effect(QUICK, { EFF_CAT_DRAW })
drawer.frequency = { HARD_PER_TURN, 1 }

-- "During your opponent's turn" gates ACTIVATION only. Once it resolves, the rule
-- fires on every battle for the rest of that turn.
function drawer:condition(e)
    return e:in_hand(YOU)
        and e:current_player() == OPPONENT
end

function drawer:cost(e)
    e:discard_self()
end

function drawer:resolve(e)
    -- A standing rule with an expiry, not a one-shot. Fires once per battle for the
    -- rest of the turn, then dies at EVENT_TURN_ENDED.
    -- The guard is load-bearing here: the draw count IS the assertion, so a final
    -- call at expiry must not sneak in an extra draw.
    e:apply_event_until(EVENT_BATTLE_ENDED, EVENT_TURN_ENDED, function(ev, until_ev)
        if until_ev then return end
        e:draw(YOU, 1)
    end)
end
