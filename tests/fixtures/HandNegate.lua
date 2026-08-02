-- HandNegate.lua — a tracked mini-Kuriboh: a QUICK monster effect activatable
-- straight from the hand during the opponent's damage calculation. Discards itself
-- as a cost, then grants "you take no battle damage" for that battle (a player
-- modifier removed when the battle ends). Exercises the full quick-from-hand path:
-- offered → activated at the window → discard cost → chain → resolve → negate.
-- Fake code (9000001x).

HandNegate = Card:new(90000016, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 300,
    def = 200,
    level = 1,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Hand Negate",
    text = "(test) Quick: discard this card; you take no battle damage from that battle.",
})

local e = HandNegate:add_effect(QUICK)
e.event = EVENT_PRE_DAMAGE_CALCULATION

function e:condition(ef)
    return ef:in_hand(YOU)
        and ef:current_player() == OPPONENT
        and ef:battle_damage() > 0
end

function e:cost(ef)
    ef:discard_self()
end

function e:resolve(ef)
    local passive = ef:add_player_modifier(YOU, MOD_NO_BATTLE_DAMAGE)
    ef:queue(EVENT_BATTLE_ENDED, { 1, ONCE }, function() ef:remove_modifier(passive) end)
end
