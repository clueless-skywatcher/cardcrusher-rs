-- BeaverWarrior.lua
-- A plain vanilla monster: no effects, just a printed record. The engine
-- harvests the `data` table (type/atk/def/level/attribute/race/text) by code.

BeaverWarrior = Card:new(32452818, {
    type = TYPE_MONSTER | TYPE_NORMAL,
    atk = 1200,
    def = 1500,
    level = 4,
    attribute = ATTRIBUTE_EARTH,
    race = RACE_BEASTWARRIOR,
    name = "Beaver Warrior",
    text = "What this creature lacks in size it makes up for in defense when battling in the prairie.",
})
