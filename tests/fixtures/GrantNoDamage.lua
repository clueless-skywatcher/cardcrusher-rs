-- GrantNoDamage.lua — on resolve, grants its controller a NoBattleDamage PLAYER
-- modifier ("you take no battle damage"). Tests the `add_player_modifier` Lua verb.
-- Fake code (9000001x).

GrantNoDamage = Card:new(90000014, {
    type = TYPE_SPELL,
    spell_type = SPELL_NORMAL,
    name = "Grant No Damage",
    text = "(test) You take no battle damage.",
})

local e = GrantNoDamage:add_effect(ACTIVATE)

function e:resolve(ef)
    ef:add_player_modifier(YOU, MOD_NO_BATTLE_DAMAGE)
end
