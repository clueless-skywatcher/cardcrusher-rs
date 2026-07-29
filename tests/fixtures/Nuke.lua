-- Nuke.lua — a NO-TARGET activate spell, for the chain tests. Destroys every
-- monster the opponent controls with no selection, so a chain test can activate
-- it without a target-selection freeze getting in the way. Fake code (9000000x).

Nuke = Card:new(90000006, {
    type = TYPE_SPELL | TYPE_NORMAL,
    name = "Nuke",
    text = "Destroy all monsters your opponent controls.",
})

local e = Nuke:add_effect(ACTIVATE, { EFF_CAT_DESTROY })

function e:resolve(effect)
    effect:destroy(effect:monster_zone(OPPONENT))
end
