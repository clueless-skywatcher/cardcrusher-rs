SPELL_NORMAL = 1
SPELL_QUICKPLAY = 2
SPELL_CONTINUOUS = 3
SPELL_FIELD = 4
SPELL_EQUIP = 5
SPELL_RITUAL = 6

-- Custom mechanic: Quest Spell
-- Quest spells don't provide any immediate benefits,
-- but reward you with some effect after a given condition is
-- met a set number of times. A Quest spell is immediately sent to 
-- the GY after it resolves. A Quest of the same original name 
-- cannot be activated while said quest's effect is already
-- active.
-- This is not necessary for now but will be implemented later.

-- E.g.
-- Timer of Greed
-- If you draw a card(s) outside of your Draw Phase twice after this
-- was activated: Draw 2 cards.
SPELL_QUEST = 7