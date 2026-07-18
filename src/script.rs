//! The card DSL runtime — the Rhai spike.
//!
//! A card is a Rhai script. Loading it runs its entry function, which describes
//! the card by calling our vocabulary. The description carries a `resolve`
//! closure that we store and run LATER — that deferred call is the whole spike.

use std::{cell::RefCell, rc::Rc};

use rhai::{self, Dynamic, Engine, FnPtr, AST};

use crate::{duel::Duel, ids::CardId, processor::DuelStatus};

/// One effect described by a card. Spike: only the deferred `resolve` closure.
pub struct EffectDef {
    /// Did the card specify a `target`? Decides whether activation asks for a
    /// target (and freezes) or resolves straight away.
    has_target: bool,
    /// The `|| { ... }` closure from the script, kept as a callable handle.
    resolve: FnPtr,
}

pub struct CardLibrary {
    /// The Rhai interpreter. Must stay alive to call `resolve` later.
    engine: Engine,
    /// The compiled card script. Must stay alive too — the closure lives in it.
    ast: Option<AST>,
    /// Effects registered by loaded cards. `Rc<RefCell<..>>` so the registered
    /// closure below and this struct can both reach the same Vec.
    effects: Rc<RefCell<Vec<EffectDef>>>,
    /// Spike observable: how many times `Destroy` fired.
    destroys: Rc<RefCell<usize>>,
    duel: Rc<RefCell<Duel>>,
    targets: Rc<RefCell<Vec<CardId>>>,
}

impl CardLibrary {
    pub fn new(duel: Rc<RefCell<Duel>>) -> Self {
        let mut engine = Engine::new();
        let effects = Rc::new(RefCell::new(vec![]));
        let destroys = Rc::new(RefCell::new(0));
        let targets = Rc::new(RefCell::new(vec![]));

        // 2nd handles to the SAME data, moved into the closures below.
        let effect_clone = effects.clone();
        let destroy_clone = destroys.clone();
        let targets_clone = targets.clone();
        let duel_clone = duel.clone();

        // `RegisterActivate(#{...})`: pull the `resolve` closure out of the map
        // and store it as an effect. e.g. RegisterActivate(#{ resolve: |d| {..} })
        engine.register_fn("RegisterActivate", move |map: rhai::Map| {
            let resolution = map.get("resolve").unwrap().clone().cast::<FnPtr>();
            let has_target = map.contains_key("target");
            effect_clone.borrow_mut().push(EffectDef {
                has_target,
                resolve: resolution,
            });
        });
        // `Destroy(duel, what)`: the referee move. Spike stub just counts it.
        engine.register_fn("Destroy", move |_what: rhai::Dynamic| {
            let mut d = duel_clone.borrow_mut();
            for id in targets_clone.borrow().iter() {
                d.remove_card(*id);
            }
            *destroy_clone.borrow_mut() += 1;
        });
        // Placeholders — just need to exist so the card parses. Real meaning later.
        engine.register_fn("PayLP", |_n: i64| Dynamic::UNIT); //     PayLP(500)
        engine.register_fn("Choose", |_a: Dynamic, _b: Dynamic| Dynamic::UNIT); // Choose(x, y)
        engine.register_fn("Monsters", |_a: Dynamic| Dynamic::UNIT); // Monsters(Opponent)
        engine.register_fn("Exactly", |_n: i64| Dynamic::UNIT); //   Exactly(1)
        engine.register_fn("GetTargets", || Dynamic::UNIT); //       GetTargets()
        engine.register_fn("Opponent", || Dynamic::UNIT);

        CardLibrary {
            engine,
            ast: None,
            effects,
            destroys,
            duel,
            targets,
        }
    }

    /// Load a card: read → compile → run its entry fn (which registers the
    /// effect) → keep the compiled script alive.
    pub fn load_file(&mut self, path: &str) -> Result<(), Box<rhai::EvalAltResult>> {
        // Read the file text and compile it into a runnable AST.
        let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let ast = self.engine.compile(&src)?;

        // Entry fn name = file stem: "cards/Example.rhai" -> "Example".
        let name = std::path::Path::new(path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        // Run it — this is what fires `RegisterActivate` and fills `effects`.
        let mut scope = rhai::Scope::new();
        self.engine.call_fn::<()>(&mut scope, &ast, &name, ())?;

        self.ast = Some(ast); // keep it alive
        Ok(())
    }

    /// How many effects loaded cards have registered.
    pub fn effect_count(&self) -> usize {
        self.effects.borrow().len()
    }

    /// How many times `Destroy` has fired (spike observable).
    pub fn destroy_count(&self) -> usize {
        *self.destroys.borrow()
    }

    /// Run a stored effect's `resolve` closure NOW — the deferred call.
    /// The closure was written at load time but runs here, later.
    pub fn resolve(&mut self, index: usize) {
        // Clone the handle so the RefCell borrow ends BEFORE we call — otherwise a
        // re-borrow inside the closure would panic (two borrows at once).
        let f = self.effects.borrow()[index].resolve.clone();
        // engine + ast must be alive here; the closure's code lives in the ast.
        let ast = self.ast.as_ref().unwrap();
        // Run it. `::<()>` = no return; `(Dynamic::UNIT,)` = the stub `duel` arg.
        // Inside, the script calls Destroy(...) → destroys += 1.
        f.call::<()>(&self.engine, ast, ()).unwrap();
    }

    /// Supply the chosen target(s) and feed the duel's inbox so it can resume.
    pub fn answer_target(&mut self, targets: Vec<CardId>) {
        *self.targets.borrow_mut() = targets;
        self.duel.borrow_mut().set_response(&[0]);
    }

    /// Activate an effect. If it has a target, ask for one (the duel freezes);
    /// otherwise resolve immediately.
    pub fn activate(&mut self, index: usize) -> DuelStatus {
        if self.effects.borrow()[index].has_target {
            let mut d = self.duel.borrow_mut();
            d.select_card(); // effect asks for a target
            d.process() // freezes → Awaiting
        } else {
            self.resolve(index); // no target → resolve now
            DuelStatus::End
        }
    }

    /// Thaw the processor, then run the effect's stored `resolve` closure.
    pub fn resume(&mut self) -> DuelStatus {
        let status = self.duel.borrow_mut().process(); // borrow ends at ';'
        self.resolve(0); // then run the effect (Destroy re-borrows the duel)
        status
    }
}
