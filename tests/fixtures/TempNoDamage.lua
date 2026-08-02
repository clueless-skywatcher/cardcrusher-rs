-- TempNoDamage.lua — grants NoBattleDamage, then removes THAT modifier (by its id)
-- when a battle ends. The Kuriboh shape, minus the quick-from-hand activation:
--   local passive = add_player_modifier(...)   -- the modifier's id
--   queue(EVENT_BATTLE_ENDED, {1, ONCE}, () -> remove_modifier(passive))
-- Fake code (9000001x).

TempNoDamage = Card:new(90000015, {
    type = TYPE_SPELL,
    spell_type = SPELL_NORMAL,
    name = "Temp No Damage",
    text = "(test) You take no battle damage until the current battle ends.",
})

local e = TempNoDamage:add_effect(ACTIVATE)

function e:resolve(ef)
    local passive = ef:add_player_modifier(YOU, MOD_NO_BATTLE_DAMAGE)
    ef:queue(EVENT_BATTLE_ENDED, { 1, ONCE }, function() ef:remove_modifier(passive) end)
end
