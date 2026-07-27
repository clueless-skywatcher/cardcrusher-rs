-- DudTrigger.lua — a test fixture: a TRIGGER effect whose `condition` is always
-- false, so it must NEVER fire even when its card is destroyed by battle. Same
-- resolve as Avenger (wipe the opponent's monsters) so a firing would be obvious.

DudTrigger = Card:new(90000002, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 1000,
    def = 1000,
    level = 4,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Dud Trigger",
    text = "(test) A trigger whose condition never holds.",
})

local dud = DudTrigger:add_effect(TRIGGER, {EFF_CAT_DESTROY})
dud.event = EVENT_BATTLE_DESTROYED

function dud:condition(effect)
    return false
end

function dud:resolve(effect)
    effect:destroy(effect:monster_zone(OPPONENT))
end
