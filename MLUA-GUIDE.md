# mlua — running Lua from Rust (for cardcrusher)

**`mlua`** = the Rust crate that embeds a Lua interpreter in our program. This doc
is the **Rust side**. The Lua language is in [LUA-PRIMER.md](LUA-PRIMER.md).

Our dependency (`Cargo.toml`):

```toml
mlua = { version = "0.12", features = ["lua54", "vendored"] }
```

- **`lua54`** = target Lua 5.4.
- **`vendored`** = build Lua from source, bundled in. No system Lua; every machine
  runs the byte-identical interpreter (matters for [determinism](#9-determinism-stop-the-gc)).

Almost every `mlua` method takes `&self` and returns `mlua::Result<T>`.

---

## 1. The VM — create and run

```rust
use mlua::Lua;

let lua = Lua::new();
lua.load("print(40 + 2)").exec()?;         // run a chunk, ignore result
let x: i64 = lua.load("40 + 2").eval()?;    // run and get a value → 42
```

- **`load(src)`** returns a `Chunk`. **`.exec()`** runs it; **`.eval::<T>()`** runs
  and returns a value.

---

## 2. Expose Rust functions to Lua (`create_function`)

Give Lua a new global word that runs Rust:

```rust
let f = lua.create_function(|_, n: i64| Ok(n * 2))?;
lua.globals().set("double", f)?;
```

Signature: `Fn(&Lua, Args) -> mlua::Result<Ret>`. We use a few globals like this
(constants, `add_effect`'s Rust hook), but **most card verbs live on the `e`
object** as UserData methods (§6), not as globals.

---

## 3. Tables — read a card object

A card is a Lua **table** (with a metatable). In Rust it's `mlua::Table`:

```rust
use mlua::{Table, Function};

let card: Table = lua.globals().get("Example")?;
let id: i64 = card.get("id")?;
let resolve: Function = card.get("resolve")?;   // pull a stage method out
```

- **`table.get::<T>(key)`** — read a field (ask for `Option<T>` if it may be nil).
- **`table.contains_key(key)`** — presence test.

---

## 4. Functions as values — store now, call later ⭐

A Lua function comes into Rust as an **`mlua::Function`**, and a card/effect is an
**`mlua::Table`**. Both are reference handles you can **keep and use later**.

```rust
struct EffectHandle {
    obj: Table,   // the effect object (holds its stage methods)
}
```

To call a method later, remember the colon rule — `obj:resolve(e)` is
`obj.resolve(obj, e)` — so pass the object as the first argument (`self`):

```rust
let resolve: Function = eff.obj.get("resolve")?;
resolve.call::<()>((eff.obj.clone(), e))?;   // (self, e)
```

> Keep the `Lua` VM alive (the `Duel` owns it) and these handles stay usable. No
> separate compiled-AST to juggle.

---

## 5. The `Card` prelude + loading a card

Cards say `Card:new(id)` and `Example:add_effect()`. So the VM must already know
`Card`. Inject that **prelude** once at VM creation:

```rust
lua.load(CARD_PRELUDE).exec()?;   // CARD_PRELUDE = the base Card class (Lua source)
```

`add_effect` (a Lua method on `Card`) creates a fresh effect object *and* hands it
to a Rust hook so the `Duel` can remember it:

```rust
// registered so Lua's `add_effect` can register the effect object with us
let register = lua.create_function(|_, eff: Table| {
    // stash `eff` on the duel (see §8 for how to reach the duel here)
    Ok(())
})?;
```

Then loading a card is just:

```rust
lua.load(&source).exec()?;   // runs Card:new + add_effect → effects registered
```

The entry-fn convention is gone — a card **registers itself** as it loads.

---

## 6. `e` — the effect object as `UserData` ⭐

`e` (the thing every stage receives) is a Rust struct exposed to Lua as an object
with methods. That's mlua **`UserData`**:

```rust
use mlua::{UserData, UserDataMethods};

struct Effect { /* shared handles: field, context, activating player… */ }

impl UserData for Effect {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("pay_lp", |_, this, n: u32| {
            // record/perform the cost
            Ok(())
        });
        methods.add_method("destroy", |_, this, targets: Table| {
            Ok(())
        });
        methods.add_method("targets", |lua, this, ()| {
            // return the chosen targets as a Lua table
            lua.create_table()
        });
    }
}
```

- **`add_method`** → the closure gets `(&Lua, &Effect, args)`.
- **`add_method_mut`** → gets `&mut Effect` (when the verb mutates the effect).

Make one and pass it into a stage call:

```rust
let e = lua.create_userdata(Effect { /* … */ })?;
resolve.call::<()>((eff.obj.clone(), e))?;   // self, e
```

Now Lua's `e:destroy(...)` runs your Rust `destroy` method. **No global verbs** —
everything hangs off `e`.

---

## 7. Coroutines — the freeze/resume bridge ⭐⭐

Run a stage on its own Lua **`Thread`** so it can pause:

```rust
use mlua::{Thread, ThreadStatus};

// bind self+e first, then a thread over that call
let thread: Thread = lua.create_thread(resolve)?;
let _: () = thread.resume((eff.obj.clone(), e.clone()))?;  // runs until yield or end

match thread.status() {
    ThreadStatus::Resumable => {
        // Lua hit coroutine.yield inside e:prompt_selection → FREEZE the duel.
        // ...host answers with `choice`...
        let _: () = thread.resume(choice)?;   // continue from the yield
    }
    ThreadStatus::Finished => { /* ran with no pause */ }
    _ => {}
}
```

| Lua coroutine | Our engine |
|---|---|
| `coroutine.yield(req)` inside `e:prompt_selection` | processor returns `DuelStatus::Awaiting` |
| `thread.status() == Resumable` | a paused effect is waiting for input |
| `thread.resume(answer)` | host answered → thaw and continue |
| `thread.status() == Finished` | effect fully resolved → pop it |

**Store the `Thread` on the paused processor unit** so resuming hits the exact
pause point.

- `resume::<R>(args)` — start/continue; returns yielded-or-final values.
- `status()` — `Resumable` / `Finished` / `Running` / `Normal` / `Error`.
- Resuming a `Finished` thread errors — check `status`.

---

## 8. Sharing the duel with `e`'s methods (the borrow trap)

The **`Duel` owns the `Lua` VM**, so `e`'s methods must **not** capture the
`Duel` (reference cycle; re-borrowing mid-call panics). Give `Effect` shared
handles instead:

**A) Shared context — "describe now, execute later":**
```rust
use std::{rc::Rc, cell::RefCell};

struct Effect { ctx: Rc<RefCell<EffectContext>> }   // records intents
// e:destroy pushes ids into ctx.to_destroy; Du: applies them after the stage runs
```

**B) mlua app-data** — stash typed state inside the VM, reach it from any method:
```rust
lua.set_app_data(EffectContext::default());
methods.add_method("destroy", |lua, _this, t: Table| {
    let mut ctx = lua.app_data_mut::<EffectContext>().unwrap();
    Ok(())
});
```

Either way: **`e`'s methods touch a shared scratchpad, never the `Duel` directly.**
The `Duel` applies the recorded changes after the stage.

---

## 9. Determinism — stop the GC

Lua is garbage-collected, and **GC timing is nondeterministic** — it could
interleave differently between machines and desync a replay (house rule #1,
`src/lib.rs`). Kill it, like EDOPro does:

```rust
lua.gc_stop();   // turn the collector OFF at VM creation
```

`vendored` pins the exact interpreter, so GC-off ⇒ every machine runs the same
bytes the same way.

---

## 10. The card lifecycle, end to end

```
Duel::new()          lua = Lua::new(); lua.gc_stop();
                     inject CARD_PRELUDE (the base Card class)     §5
                     register hooks (add_effect) + make e a UserData type   §2,§6
load_card("X.lua")   lua.load(src).exec()
                     → Card:new + add_effect → effect objects stored   §3,§4,§5
activate(effect)     run condition/cost; make a Thread over the stage   §7
   ↳ target yields    e:prompt_selection → coroutine.yield → Resumable → FREEZE
answer + resume      thread.resume(choice) → Lua continues → e:destroy records   §8
                     thread Finished → Duel applies the records (send_to GY)
```

Build order lives in **[PHASE-LUA.md](PHASE-LUA.md)**.
