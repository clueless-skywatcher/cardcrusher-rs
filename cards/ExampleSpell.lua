-- ExampleSpell.lua
-- "Pay 500 LP, then destroy 1 monster your opponent controls."
--
-- A card holds effects. Add each with add_effect(), then write only the stages
-- you change. `e` is the effect at runtime: call verbs on it (e:pay_lp, ...).

-- A Normal Spell. Its printed record (type + text) is harvested by the engine.
ExampleSpell = Card:new(12345678, {
    type = TYPE_SPELL | TYPE_NORMAL,
    name = "Example Spell",
    text = "Pay 500 LP, then destroy 1 monster your opponent controls.",
})

local activate = ExampleSpell:add_effect(ACTIVATE, {EFF_CAT_DESTROY})

function activate:cost(effect)
    effect:pay_lp(500) -- Pay LP
end

function activate:target(effect)
    -- Choose exactly one monster in the opponent's monster zone
    effect:prompt_selection(effect:monster_zone(OPPONENT), 1)
end

function activate:resolve(effect)
    -- Destroy the targets chosen in target(effect)
    effect:destroy(effect:targets())
end
