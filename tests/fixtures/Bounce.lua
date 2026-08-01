-- Bounce.lua — a normal-spell fixture for the `send` verb: return 1 monster the
-- opponent controls to the hand. Proves `send(card, ZONE_HAND)` relocates a card
-- (and that a "send" is not a "destroy"). Fake code (9000000x).

Bounce = Card:new(90000007, {
    type = TYPE_SPELL,
    spell_type = SPELL_NORMAL,
    name = "Bounce",
    text = "Return 1 monster your opponent controls to the hand.",
})

local e = Bounce:add_effect(ACTIVATE)

function e:target(effect)
    effect:prompt_selection(effect:monster_zone(OPPONENT), 1)
end

function e:resolve(effect)
    effect:send(effect:targets(), ZONE_HAND)
end
