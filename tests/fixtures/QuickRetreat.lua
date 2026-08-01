-- QuickRetreat.lua — a QUICK-PLAY spell (Spell Speed 2, no target): return all
-- monsters YOU control to the hand. Used to make chain LIFO observable — chained
-- onto an opponent's board-wipe, it whisks your monster to safety *first*, so the
-- wipe re-reads an empty field. Fake code (9000000x).

QuickRetreat = Card:new(90000009, {
    type = TYPE_SPELL,
    spell_type = SPELL_QUICKPLAY,
    name = "Quick Retreat",
    text = "Return all monsters you control to the hand.",
})

local e = QuickRetreat:add_effect(ACTIVATE)

function e:resolve(effect)
    effect:send(effect:monster_zone(YOU), ZONE_HAND)
end
