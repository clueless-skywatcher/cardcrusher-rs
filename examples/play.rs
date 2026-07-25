//! A tiny terminal harness to *play* the engine.
//!
//! Run with:  `cargo run --example play`
//!
//! It's a hotseat: whoever's turn it is acts from this terminal. It drives the
//! normal `process()` / `Awaiting` / `set_response` loop — the same protocol a
//! real front-end would use — and renders the board by querying the `Duel`.
//!
//! Scope note: this exercises what's built (turns, the Main-Phase menu, summon,
//! and menu-driven effect activation with target selection). No battle, no
//! chains — those are later phases.

use std::io::{self, Write};

use cardcrusher::card::Card;
use cardcrusher::duel::{Duel, Winner};
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{
    CMD_ACTIVATE, CMD_NEXT_PHASE, CMD_SUMMON, MSG_NEW_TURN, MSG_PHASE_BATTLE, MSG_PHASE_DRAW,
    MSG_PHASE_END, MSG_PHASE_MAIN1, MSG_PHASE_MAIN2, MSG_PHASE_STANDBY, MSG_SELECT_CARD,
    MSG_SELECT_IDLECMD, MSG_STARTUP,
};

fn main() {
    let mut duel = setup();
    duel.start();
    let mut seen = 0usize; // how many outbox messages we've already printed

    println!("\n=== cardcrusher — hotseat demo ===");

    loop {
        let status = duel.process();
        print_new_messages(&duel, &mut seen);

        match status {
            DuelStatus::End => {
                announce_result(&duel);
                break;
            }
            // process() only ever returns Awaiting or End at the top level.
            _ => match duel.messages().last().copied() {
                Some(MSG_SELECT_IDLECMD) => main_phase_menu(&mut duel),
                Some(MSG_SELECT_CARD) => select_target(&mut duel),
                _ => duel.set_response(&[0]), // shouldn't happen; keep moving
            },
        }
    }
}

/// A fixed opening position: player 0 holds the Example spell + a monster to
/// summon; player 1 has a monster on the field to target. Small decks so turns
/// don't deck-out instantly (they will, eventually — that ends the game).
fn setup() -> Duel {
    let mut duel = Duel::new();
    duel.load_card("cards/Example.lua")
        .expect("cards/Example.lua should load");

    // A mix of named monsters, so drawn/summoned cards are clearly different.
    let deck = [1001u32, 1002, 1003, 1004];
    for i in 0..8 {
        duel.add_to_deck(0, Card::new(deck[i % deck.len()]));
        duel.add_to_deck(1, Card::new(deck[(i + 1) % deck.len()]));
    }
    duel.add_to_hand(0, Card::new(12345678)); // Example Spell
    duel.add_to_hand(0, Card::new(1001)); // Kuriboh — a monster to summon

    let foe = duel.add_card(Card::new(1002)); // Beaver Warrior on the opponent's field
    duel.place(1, foe, Zone::MonsterZone);
    duel
}

// ===== Rendering =========================================================

fn card_name(code: u32) -> &'static str {
    match code {
        12345678 => "Example Spell",
        11111111 => "CantActivate",
        10312660 => "You're in Danger!",
        1001 => "Kuriboh",
        1002 => "Beaver Warrior",
        1003 => "Feral Imp",
        1004 => "Mystical Elf",
        _ => "Monster",
    }
}

/// The Example Spell is our only Spell; everything else in the demo is a
/// summonable monster.
fn is_monster(code: u32) -> bool {
    code != 12345678
}

fn name_of(duel: &Duel, id: cardcrusher::ids::CardId) -> &'static str {
    card_name(duel.get_card(id).map(|c| c.code).unwrap_or(0))
}

fn print_new_messages(duel: &Duel, seen: &mut usize) {
    let msgs = duel.messages();
    for &m in &msgs[*seen..] {
        if let Some(label) = phase_label(m) {
            println!("── {label} ──");
        }
    }
    *seen = msgs.len();
}

fn phase_label(m: u8) -> Option<&'static str> {
    Some(match m {
        MSG_STARTUP => "Duel start",
        MSG_NEW_TURN => "New turn",
        MSG_PHASE_DRAW => "Draw Phase",
        MSG_PHASE_STANDBY => "Standby Phase",
        MSG_PHASE_MAIN1 => "Main Phase 1",
        MSG_PHASE_BATTLE => "Battle Phase",
        MSG_PHASE_MAIN2 => "Main Phase 2",
        MSG_PHASE_END => "End Phase",
        _ => return None, // MSG_SELECT_* are prompts, not phases
    })
}

fn show_board(duel: &Duel, player: usize) {
    let opp = 1 - player;
    let cards = |ids: Vec<cardcrusher::ids::CardId>| {
        if ids.is_empty() {
            "(empty)".to_string()
        } else {
            ids.iter()
                .map(|&id| name_of(duel, id))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let hand = (0..duel.hand_count(player))
        .filter_map(|i| duel.hand_card(player, i))
        .map(|id| name_of(duel, id))
        .collect::<Vec<_>>()
        .join(", ");

    println!(
        "\n┌─ Player {player}'s turn ─ LP {} vs {} ─┐",
        duel.life_points(player),
        duel.life_points(opp)
    );
    println!("│ Opponent field: {}", cards(duel.monster_zone(opp)));
    println!("│ Your field:     {}", cards(duel.monster_zone(player)));
    println!(
        "│ Your hand:      {}",
        if hand.is_empty() {
            "(empty)".into()
        } else {
            hand
        }
    );
    println!("└{}", "─".repeat(30));
}

// ===== Prompts ===========================================================

fn main_phase_menu(duel: &mut Duel) {
    let player = *duel.turn_history().last().unwrap_or(&0);
    show_board(duel, player);

    // Build the option list: (label, response bytes).
    let mut options: Vec<(String, Vec<u8>)> =
        vec![("Go to next phase".into(), vec![CMD_NEXT_PHASE])];

    // Summon: monster (code 0) cards in hand.
    for i in 0..duel.hand_count(player) {
        if let Some(id) = duel.hand_card(player, i) {
            if duel.get_card(id).map(|c| c.code).is_some_and(is_monster) {
                options.push((
                    format!("Summon {}", name_of(duel, id)),
                    vec![CMD_SUMMON, i as u8],
                ));
            }
        }
    }
    // Activate: whatever the engine says is activatable right now.
    for (opt, (card, _slot)) in duel.activatable_effects(player).into_iter().enumerate() {
        options.push((
            format!("Activate {}", name_of(duel, card)),
            vec![CMD_ACTIVATE, opt as u8],
        ));
    }

    println!("\nChoose an action:");
    for (i, (label, _)) in options.iter().enumerate() {
        println!("  [{i}] {label}");
    }
    let choice = read_index("Action", options.len());
    let response = options[choice].1.clone();
    duel.set_response(&response);
}

fn select_target(duel: &mut Duel) {
    let cands = duel.candidates();
    println!("\nPick a target:");
    for (i, &id) in cands.iter().enumerate() {
        println!("  [{i}] {}", name_of(duel, id));
    }
    let choice = read_index("Target", cands.len().max(1));
    duel.set_response(&[choice as u8]);
}

fn announce_result(duel: &Duel) {
    match duel.result() {
        Some(Winner::Player(p)) => {
            println!("\n🏆 Player {p} wins — {:?}", duel.win_reason());
        }
        Some(Winner::Draw) => println!("\n🤝 It's a draw."),
        None => println!("\nGame over."),
    }
}

// ===== Input =============================================================

/// Read an index in `0..count` from stdin, re-prompting on bad input.
fn read_index(what: &str, count: usize) -> usize {
    loop {
        print!("{what} [0-{}]: ", count.saturating_sub(1));
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            std::process::exit(0); // EOF (e.g. piped input ended) → quit cleanly
        }
        match line.trim().parse::<usize>() {
            Ok(n) if n < count => return n,
            _ => println!("  (enter a number 0..{})", count.saturating_sub(1)),
        }
    }
}
