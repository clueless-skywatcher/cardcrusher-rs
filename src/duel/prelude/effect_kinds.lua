-- Effect kinds — how/where an effect is activated (the first arg to add_effect).
ACTIVATE = 0 -- a Spell/Trap card's activation (from hand / set zone)
IGNITION = 1 -- a manual effect on a card you control, on your Main Phase
QUICK    = 2 -- quick effect (needs the chain engine)
TRIGGER  = 3 -- fires on an event (the event engine)
