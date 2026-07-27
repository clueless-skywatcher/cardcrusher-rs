-- Kuriboh.lua — a vanilla demo monster (its real effect isn't modelled yet).

Kuriboh = Card:new(40640057, {
    type = TYPE_MONSTER | TYPE_EFFECT,
    atk = 300,
    def = 200,
    level = 1,
    attribute = ATTRIBUTE_DARK,
    race = RACE_FIEND,
    name = "Kuriboh",
    text = "During damage calculation, if your opponent's monster attacks (Quick Effect): You can discard this card; you take no battle damage from that battle.",
})
