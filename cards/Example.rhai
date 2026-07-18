// Example.rhai — the M5 spike card.
//
// In words: "Pay 500 LP, then destroy 1 monster your opponent controls."
//
// The entry point is named after the card (here: `Example`). The loader calls
// the function matching the card's name. It runs ONCE at load and only DESCRIBES
// the card — it does not do anything yet. The `resolve` closure is stored by the
// engine and run LATER, when the effect actually resolves. Proving that deferred
// call works is the whole point of Milestone 5.

fn Example() {
    RegisterActivate(#{
        cost:    PayLP(500),
        target:  Choose(Monsters(Opponent()), Exactly(1)),
        resolve: || { Destroy(GetTargets()); }
    });
}
