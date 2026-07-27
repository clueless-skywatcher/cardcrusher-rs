-- MysticalElf.lua — a vanilla demo monster; a defensive wall (2000 DEF).

MysticalElf = Card:new(15025844, {
    type = TYPE_MONSTER | TYPE_NORMAL,
    atk = 800,
    def = 2000,
    level = 4,
    attribute = ATTRIBUTE_LIGHT,
    race = RACE_SPELLCASTER,
    name = "Mystical Elf",
    text = "A delicate elf with little offense but a terrific defense, backed by mystical power.",
})
