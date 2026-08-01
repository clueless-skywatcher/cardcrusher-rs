-- TriCatSpell.lua — test fixture: one effect that advertises MULTIPLE categories,
-- mirroring EDOPro's Dracotail Sting (SetCategory(REMOVE + TODECK + DRAW) on a
-- single effect). We store them as a list rather than an OR'd bitmask.

TriCatSpell = Card:new(90000004, {
    type = TYPE_SPELL,
    spell_type = SPELL_NORMAL,
    name = "Tri-Cat Spell",
    text = "(test) One effect advertising three categories.",
})

TriCatSpell:add_effect(ACTIVATE, { EFF_CAT_BANISH, EFF_CAT_TO_DECK, EFF_CAT_DRAW })
