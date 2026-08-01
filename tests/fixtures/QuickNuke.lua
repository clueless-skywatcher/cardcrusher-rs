-- QuickNuke.lua — a QUICK-PLAY spell fixture (Spell Speed 2, no target). Same
-- board-wipe as Nuke, but — being quick-play — it can be activated in RESPONSE to
-- a chain. Used to test the chainability gate. Fake code (9000000x).

QuickNuke = Card:new(90000008, {
    type = TYPE_SPELL,
    spell_type = SPELL_QUICKPLAY,
    name = "Quick Nuke",
    text = "Destroy all monsters your opponent controls.",
})

local e = QuickNuke:add_effect(ACTIVATE)

function e:condition(effect)
    -- `#` is the length: only activatable if the opponent controls ≥ 1 monster.
    return #effect:monster_zone(OPPONENT) >= 1
end

function e:resolve(effect)
    effect:destroy(effect:monster_zone(OPPONENT))
end
