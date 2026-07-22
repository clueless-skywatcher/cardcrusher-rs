# Lua in 10 minutes (for cardcrusher)

**Lua** = a tiny scripting language built to run *inside* another program. We use
it so a **card** is a little script, not hard-coded Rust.

This doc is the **language**. The Rust side (how we run Lua from Rust) is in
[MLUA-GUIDE.md](MLUA-GUIDE.md).

> Read in order. Each section is a thing our cards actually use. Two stars matter
> most: **§6 OOP** (how a card is an object) and **§8 coroutines** (why we chose
> Lua). Don't skip them.

---

## 0. The 30-second shape of a card

```lua
Example = Card:new(12345678)           -- a card is an object

local activate = Example:add_effect()  -- a card HAS effects; add each one

function activate:cost(e)    e:pay_lp(500) end
function activate:target(e)  e:prompt_selection(e:monster_zone(OPPONENT), 1) end
function activate:resolve(e) e:destroy(e:targets()) end
```

If you get this snippet, you get our DSL. Everything below explains one piece.

---

## 1. Values & variables

The types we care about: **number, string, boolean, `nil`, function, table**.

```lua
local x = 42          -- number (we keep to integers)
local name = "Dark Hole"
local ok = true
local nothing = nil   -- "no value" — Lua's null
```

- **`local`** = scoped to this block. **Always use `local`.** Leaving it off makes
  a *global* that leaks across every card.
- **`nil`** = absence. Reading a missing table key gives `nil`, not an error.

---

## 2. `nil` and truthiness (a gotcha)

Only **`false` and `nil`** are falsy. **Everything else is true — including `0`.**

```lua
if 0 then print("yes") end   -- prints! 0 is truthy in Lua
```

> ⚠️ Coming from C/Rust, this bites. `0` is not false here.

---

## 3. Functions are values

A function is a value: store it, pass it, put it in a table.

```lua
local double = function(n) return n * 2 end
```

Our effect stages (`cost`, `target`, `resolve`) are functions attached to an
object — stored now, called later by the engine.

---

## 4. Tables — the *only* data structure

A **table** is Lua's everything: dictionary, array, object. Curly braces build one.

**As a dictionary:**
```lua
local card = { name = "Dark Hole", atk = 0 }
card.name        -- "Dark Hole"
```

**As an array (1-indexed!):**
```lua
local zones = { "Deck", "Hand", "GY" }
zones[1]         -- "Deck"   (Lua counts from 1, not 0)
#zones           -- 3        (# = length)
```

> ⚠️ **1-indexed.** When Rust hands Lua a list to pick from, index 1 is the first
> element. This is the classic Lua off-by-one.

---

## 5. `self` and the colon `:` (methods)

A method is a function that works on an object. Two rules:

**Define with `:`** — you get a hidden `self` (the object it was called on):
```lua
function activate:cost(e)   -- `self` == activate, `e` is the real argument
    e:pay_lp(500)
end
```

**Call with `:`** — passes the object as `self` automatically:
```lua
e:pay_lp(500)      -- means  e.pay_lp(e, 500)
```

That's the whole trick: **`obj:method(args)` = `obj.method(obj, args)`.** The
colon hides the `self`.

---

## 6. ⭐ OOP: objects & inheritance (metatables)

Lua has no `class` keyword. Instead, a table can fall back to another table for
anything it doesn't have — via a **metatable** with an **`__index`**. That
fallback *is* inheritance. (This is straight from *Programming in Lua*, ch. 16.)

The base class every card builds on (the engine gives you this — you don't write
it):
```lua
Card = {}
Card.__index = Card

function Card:new(id)                 -- constructor
    local o = setmetatable({}, self)  -- new object; fall back to Card
    o.id = id
    return o
end

-- default effect stages: a card overrides only what it changes
function Card:condition(e) return true end   -- activatable by default
function Card:cost(e)      end               -- free by default
function Card:target(e)    end               -- no target by default
function Card:resolve(e)   end               -- does nothing by default
```

So when you write:
```lua
Example = Card:new(12345678)
function Example:resolve(e) e:destroy(e:targets()) end
```
`Example` gets `new`, `condition`, `cost`, etc. **for free** from `Card`, and you
only spell out `resolve`. That's the payoff of OOP here: **write only what's
different.**

> Mental model: ask `Example` for `condition` → it doesn't have its own → Lua
> follows `__index` to `Card` and finds the default. Override = give `Example` its
> own, and it wins.

---

## 7. Closures — functions that remember

A function made inside another **captures** the surrounding locals.

```lua
function activate:resolve(e)
    local chosen = pick_one()
    e:queue(END_PHASE_BEGAN, function() e:destroy(chosen) end)  -- remembers `chosen`
end
```

We lean on this: the little `function() ... end` runs *later* but still knows
`chosen` and `e`.

---

## 8. ⭐⭐ Coroutines — pause and resume (the reason for Lua)

A **coroutine** stops in the middle, hands control back, and later **picks up
exactly where it left off** — locals intact.

```lua
local co = coroutine.create(function()
    local pick = coroutine.yield("choose a monster")  -- ⏸ send request OUT
    print("you picked " .. pick)                       -- resumes here
end)

local _, request = coroutine.resume(co)   -- request == "choose a monster"
coroutine.resume(co, "Blue-Eyes")         -- feeds the answer back IN → prints it
```

- **`yield(x)`** — pause; send `x` out to whoever resumed us.
- **`resume(co, y)`** — un-pause; `y` becomes what `yield` returns inside.

### Why this is the whole ballgame

A card that says *"destroy 1 monster your opponent controls"* must **stop and ask
which one**, then continue:

```lua
function activate:target(e)
    e:prompt_selection(e:monster_zone(OPPONENT), 1)  -- ⏸ asks the player
end
function activate:resolve(e)
    e:destroy(e:targets())                            -- runs after the answer
end
```

`prompt_selection` does `coroutine.yield` under the hood — the duel **freezes**,
the host asks the human, `resume` feeds the choice back. No mid-effect question is
possible without this.

> The engine's freeze/resume and Lua's `yield`/`resume` are the **same pause**,
> wired together.

---

## 9. Syntax you'll actually see

```lua
-- comments start with two dashes
local s = "Dark" .. " Hole"      -- `..` joins strings
if a and b then ... elseif c then ... else ... end
for i = 1, 3 do ... end           -- numeric loop, INCLUSIVE of 3
x == y      x ~= y                -- equal / NOT equal (`~=`, not `!=`)
a >= b                            -- greater-or-equal (NOT `=>`)
not done                          -- boolean negation is the word `not`
```

Blocks end with **`end`** (no braces). No semicolons needed.

---

## 10. How it maps to `cards/Example.lua`

| Card piece | Section |
|---|---|
| `Example = Card:new(id)` | §6 — a card is an object inheriting `Card` |
| `Example:add_effect()` | §4/§6 — the card holds effect objects |
| `function activate:cost(e) ... end` | §5 — a method; `e` is the effect |
| `e:pay_lp`, `e:destroy`, `e:monster_zone` | §5 — verbs are methods on `e` |
| picking a target mid-effect | §8 — the coroutine yield/resume bridge |
| stages you *don't* write (`condition`) | §6 — inherited from `Card` |

Next: **[MLUA-GUIDE.md](MLUA-GUIDE.md)** — how Rust runs all this.
