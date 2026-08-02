-- The card DSL base classes (Effect, Card) and the effect verbs. Loaded after the
-- constant tables (players / effect_kinds / categories / card_types / attributes /
-- races), before any card — so those names exist when a card is loaded.

-- Base class for effects. A card's effect inherits these default stages and
-- overrides only the ones it changes.
Effect = {}
Effect.__index = Effect
function Effect:condition(e) return true end -- activatable by default

function Effect:cost(e) end                  -- free by default

function Effect:target(e) end                -- no target by default

function Effect:resolve(e) end               -- does nothing by default

-- Effect verbs: thin wrappers over the engine's Rust hooks. They read/write the
-- shared effect context, so what a stage does is applied to the real duel.
function Effect:targets() return effect_targets() end

function Effect:destroy(cards) effect_destroy(cards) end

-- Move card(s) to a zone (ZONE_HAND / ZONE_DECK / ZONE_GY / ZONE_BANISHMENT / …).
-- A plain relocation — NOT a destruction (no "destroyed" trigger fires).
function Effect:send(cards, zone) effect_send(cards, zone) end

function Effect:pay_lp(n) effect_pay_lp(n) end

-- Ask the host to choose `count` cards from `candidates`. This PAUSES the whole
-- duel (coroutine.yield) until the host answers; the engine records the candidate
-- set (so it can reject an empty one and map the picked index back to a card).
-- Because it's plain Lua, the stage suspends linearly.
function Effect:prompt_selection(candidates, count)
    effect_prompt_selection(candidates)
    return coroutine.yield(count)
end

-- The monsters a player controls (`who` relative to the activating player).
function Effect:monster_zone(who) return effect_monster_zone(who) end

-- Is THIS card (the one running the effect) in `who`'s hand? (`who` relative to
-- the activating player.) Lets a hand-only effect gate on its own location.
function Effect:in_hand(who) return effect_in_hand(who) end

-- Declare "discard this card" as a cost: when the cost is paid, this very card is
-- sent from the hand to the GY.
function Effect:discard_self() effect_discard_self() end

-- The battle damage the activating player is about to take at the current damage
-- step (0 outside a damage-calculation window).
function Effect:battle_damage() return effect_battle_damage() end

-- Whose turn it is, relative to the activator: YOU on your own turn, OPPONENT on
-- theirs. (Kuriboh checks `current_player() == OPPONENT` — "the opponent attacked".)
function Effect:current_player() return effect_current_player() end

-- A detail of the event that fired this trigger, by (event code, key) — e.g.
-- get_event_detail(EVENT_DESTROYED, "destroyed_card"). The code guards it: nil
-- unless the current event matches. Each event type defines its own detail keys.
function Effect:get_event_detail(code, key) return effect_get_event_detail(code, key) end

-- Grant a PLAYER modifier: a MOD_* code (+ optional value), on `who` relative to
-- the activator. Returns the new modifier's id (for remove_modifier later).
function Effect:add_player_modifier(who, code, value) return effect_add_player_modifier(who, code, value) end

-- Remove the single modifier with `id`.
function Effect:remove_modifier(id) effect_remove_modifier(id) end

-- Run `fn` when `event` fires, per the { count, period } frequency. Pass `fn` as a
-- closure so it can capture locals (a bound method `e:method` isn't a valid value).
-- `fn` receives the event that fired it.
function Effect:queue(event, freq, fn) effect_queue(event, freq, fn) end

-- Run `fn` EVERY time `on_event` fires, until `until_event` arrives — a standing
-- rule with an expiry rather than a countdown ("this turn, each time ...").
-- `fn` gets (ev, until_ev), exactly one of which is non-nil: `ev` on each repeat,
-- `until_ev` on the single final call as the rule is removed (a teardown hook).
-- The engine remembers who registered it, so YOU/OPPONENT stay correct throughout.
function Effect:apply_event_until(on_event, until_event, fn)
    effect_apply_event_until(on_event, until_event, fn)
end

-- Draw `n` cards for `who` (YOU/OPPONENT, relative to the activating player).
function Effect:draw(who, n) effect_draw(who, n) end

-- Base class for cards.
Card         = {}
Card.__index = Card
-- `data` (optional) is the card's printed record: { type, atk, def, level,
-- attribute, race, name, text }. The engine harvests it (register_card) by id.
function Card:new(id, data)
    register_card(id, data or {})
    return setmetatable({ id = id }, self)
end

-- Make a fresh effect (inheriting Effect's defaults) and hand it to the engine.
--   kind     — ACTIVATE / IGNITION / QUICK / TRIGGER (how/where it acts).
--   category — an EFF_CAT_* bitmask of WHAT it does (optional; 0 if omitted).
-- Both ride on the effect table for the engine to read.
function Card:add_effect(kind, category)
    local effect = setmetatable({ kind = kind, category = category or {} }, Effect)
    register_effect(self.id, effect)
    return effect
end
