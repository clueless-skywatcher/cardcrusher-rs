//! An interactive terminal harness to *play* the engine.
//!
//! Run with:  `cargo run --example play`   (needs a real terminal)
//!
//! It's a hotseat: whoever's turn it is acts from this terminal. It drives the
//! normal `process()` / `Awaiting` / `set_response` loop — the same protocol a
//! real front-end would use — and renders the board by querying the `Duel`.
//!
//! Controls — mouse or keyboard:
//!   Click        pick a menu item, or a card on the board (attacker / target)
//!   Hover        preview the card under the mouse in the details panel
//!   Right-click  a GY pile → view all the cards in it
//!   Scroll       move the menu highlight
//!   Tab · arrows · Enter · q   keyboard focus / navigation / select / quit
//!
//! The board shows each player's hand (yours face-up, the opponent's face-down),
//! their Monster + Spell/Trap zones, and a GY pile per side (right-click to open).
//!
//! Rendering is **double-buffered**: each frame is composed into an off-screen
//! cell grid ([`Screen`]), then only the rows that changed since the last frame
//! are written to the terminal — no full-screen clear, so no flicker.
//!
//! Scope note: exercises what's built — turns, summon, effect activation with
//! target selection, the Battle Phase, and the full chain (response windows,
//! spell speed, LIFO, triggers on the chain, SEGOC).

use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{cursor, queue, terminal};

use cardcrusher::duel::{Duel, Winner};
use cardcrusher::ids::CardId;
use cardcrusher::position::Position;
use cardcrusher::processor::DuelStatus;
use cardcrusher::zone::Zone;
use cardcrusher::{
    CMD_ACTIVATE, CMD_ATTACK, CMD_NEXT_PHASE, CMD_PASS, CMD_RESPONSE, CMD_SUMMON, MSG_NEW_TURN,
    MSG_PHASE_BATTLE, MSG_PHASE_DRAW, MSG_PHASE_END, MSG_PHASE_MAIN1, MSG_PHASE_MAIN2,
    MSG_PHASE_STANDBY, MSG_SELECT_ATTACK_TARGET, MSG_SELECT_BATTLECMD, MSG_SELECT_CARD,
    MSG_SELECT_CHAIN, MSG_SELECT_IDLECMD, MSG_SELECT_YESNO, MSG_STARTUP,
};

/// How long an automatic beat lingers on screen so a hotseat player can follow
/// what the engine just did (a phase change, a resolved battle, a trigger).
const BEAT: Duration = Duration::from_millis(450);

fn main() {
    let mut duel = setup();
    duel.start();

    {
        let _guard = TerminalGuard::new();
        let mut ui = Ui::new();
        // Drive ONE step at a time (not process(), which runs silently to the next
        // prompt). That lets us pause on internal steps — phase changes, resolved
        // battles, trigger effects — so the player can watch them happen.
        loop {
            match duel.step() {
                DuelStatus::End => break,
                // A prompt is waiting → run the matching menu.
                DuelStatus::Awaiting => match duel.messages().last().copied() {
                    Some(MSG_SELECT_YESNO) => yesno_menu(&mut duel, &mut ui),
                    Some(MSG_SELECT_IDLECMD) => main_phase_menu(&mut duel, &mut ui),
                    Some(MSG_SELECT_BATTLECMD) => battle_phase_menu(&mut duel, &mut ui),
                    Some(MSG_SELECT_ATTACK_TARGET) => select_attack_target(&mut duel, &mut ui),
                    Some(MSG_SELECT_CARD) => select_target(&mut duel, &mut ui),
                    Some(MSG_SELECT_CHAIN) => chain_response_menu(&mut duel, &mut ui),
                    _ => duel.set_response(&[0]), // shouldn't happen; keep moving
                },
                // The engine did internal work. If the board actually changed,
                // linger on it for a beat; skip invisible bookkeeping steps.
                DuelStatus::Continue => {
                    let title = format!("… {}", current_phase_label(&duel));
                    if render(&duel, &mut ui, &title, &[]) {
                        sleep(BEAT);
                    }
                }
            }
        }
        game_over(&duel, &mut ui);
    } // guard drops here → terminal restored

    announce_result(&duel);
}

/// A fixed opening position: player 0 holds the Example spell + a monster to
/// summon; player 1 has a monster on the field to target. Small decks so turns
/// don't deck-out instantly (they will, eventually — that ends the game).
// The codes the demo builds a deck from; all data comes from the loaded scripts.
const KURIBOH: u32 = 40640057;
const BEAVER_WARRIOR: u32 = 32452818;
const FERAL_IMP: u32 = 41392891;
const MYSTICAL_ELF: u32 = 15025844;
const EXAMPLE_SPELL: u32 = 12345678;
const OPTIONAL_AVENGER: u32 = 90000005;
const QUICK_NUKE: u32 = 90000008; // a Quick-Play spell (Spell Speed 2)

fn setup() -> Duel {
    let mut duel = Duel::new();
    load_all_cards(&mut duel);

    // A small mixed deck, built purely from codes — every stat/name/text is
    // whatever the loaded `.lua` scripts declared.
    let deck = [KURIBOH, BEAVER_WARRIOR, FERAL_IMP, MYSTICAL_ELF];
    for i in 0..8 {
        let a = duel.make_card(deck[i % deck.len()]);
        duel.add_to_deck(0, a);
        let b = duel.make_card(deck[(i + 1) % deck.len()]);
        duel.add_to_deck(1, b);
    }

    let spell = duel.make_card(EXAMPLE_SPELL);
    duel.add_to_hand(0, spell);
    let imp = duel.make_card(FERAL_IMP); // a monster to summon
    duel.add_to_hand(0, imp);
    let kuriboh = duel.make_card(KURIBOH);
    duel.add_to_hand(0, kuriboh);
    // Optional Avenger (1000 ATK): summon it, ram it into the stronger Beaver
    // Warrior below, and its OPTIONAL "when destroyed" trigger fires — the
    // yes/no prompt. Yes wipes the opponent's board.
    let avenger = duel.make_card(OPTIONAL_AVENGER);
    duel.add_to_hand(0, avenger);
    // P1 holds a Quick-Play (Spell Speed 2) so they can CHAIN in response to P0's
    // activations — exercising the response window + LIFO resolution.
    let quick = duel.make_card(QUICK_NUKE);
    duel.add_to_hand(1, quick);

    let foe = duel.make_card(FERAL_IMP); // on the opponent's field
    let foe = duel.add_card(foe);
    duel.place(1, foe, Zone::MonsterZone);
    // Face-up attack so an attack into it is ATK-vs-ATK (1200 > 1000) and kills
    // the Avenger — a face-down/positionless wall wouldn't destroy the attacker.
    duel.change_position(foe, Position::FaceUpAttack);
    duel
}

/// Load every card script at startup — the demo's "card database". Real cards
/// live in `cards/`; the made-up demo/test cards (Example Spell, Optional
/// Avenger, …) live in `tests/fixtures/`. Sorted before loading so the load
/// order (and thus effect registration) is deterministic.
fn load_all_cards(duel: &mut Duel) {
    let mut paths: Vec<std::path::PathBuf> = ["cards", "tests/fixtures"]
        .iter()
        .flat_map(|dir| std::fs::read_dir(dir).unwrap_or_else(|_| panic!("read {dir}/")))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("lua"))
        .collect();
    paths.sort();
    for path in paths {
        let p = path.to_str().expect("card path is valid UTF-8");
        duel.load_card(p)
            .unwrap_or_else(|e| panic!("failed to load {p}: {e}"));
    }
}

/// The player acting at the current prompt — the one whose perspective the board
/// is drawn from. Normally the turn player, but while a chain-response window is
/// open it's the **responder**, so the board flips to *their* side (each player
/// sees their own hand at the bottom when it's their decision).
fn current(duel: &Duel) -> usize {
    match duel.messages().last() {
        Some(&MSG_SELECT_CHAIN) => duel.chain_responder(),
        _ => *duel.turn_history().last().unwrap_or(&0),
    }
}

// ===== Double-buffered screen ============================================

#[derive(Clone, Copy, PartialEq)]
struct Cell {
    ch: char,
    fg: Color,
    bold: bool,
    italic: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: Color::Reset,
            bold: false,
            italic: false,
        }
    }
}

/// An off-screen character grid. Draw a whole frame into `cur`, then `flush`
/// writes only the rows that differ from the previously shown frame (`prev`).
struct Screen {
    w: usize,
    h: usize,
    cur: Vec<Cell>,
    prev: Vec<Cell>,
}

impl Screen {
    fn new() -> Self {
        let (w, h) = terminal::size().unwrap_or((100, 40));
        // Use the REAL terminal size (a small floor avoids a degenerate buffer).
        // Inflating it would place rows below the visible area and corrupt output.
        let (w, h) = ((w as usize).max(40), (h as usize).max(18));
        Screen {
            w,
            h,
            cur: vec![Cell::default(); w * h],
            // A sentinel that no real cell equals, so the first flush draws all.
            prev: vec![
                Cell {
                    ch: '\0',
                    ..Cell::default()
                };
                w * h
            ],
        }
    }

    /// Blank the working buffer for a fresh frame.
    fn begin(&mut self) {
        self.cur.iter_mut().for_each(|c| *c = Cell::default());
    }

    fn put(&mut self, x: u16, y: u16, s: &str, fg: Color) {
        self.put_styled(x, y, s, fg, false, false);
    }

    fn put_bold(&mut self, x: u16, y: u16, s: &str, fg: Color) {
        self.put_styled(x, y, s, fg, true, false);
    }

    fn put_styled(&mut self, x: u16, y: u16, s: &str, fg: Color, bold: bool, italic: bool) {
        let (x, y) = (x as usize, y as usize);
        if y >= self.h {
            return;
        }
        for (i, ch) in s.chars().enumerate() {
            let cx = x + i;
            if cx >= self.w {
                break;
            }
            self.cur[y * self.w + cx] = Cell {
                ch,
                fg,
                bold,
                italic,
            };
        }
    }

    /// Write only the changed rows to the terminal, grouping equal-style runs.
    /// Returns whether any row differed from the previous frame (used to decide
    /// if an automatic beat is worth pausing on).
    fn flush(&mut self, out: &mut impl Write) -> bool {
        let mut changed = false;
        for y in 0..self.h {
            let row = &self.cur[y * self.w..(y + 1) * self.w];
            if row == &self.prev[y * self.w..(y + 1) * self.w] {
                continue; // unchanged → don't touch it (this is what kills flicker)
            }
            changed = true;
            let _ = queue!(out, cursor::MoveTo(0, y as u16));
            let mut i = 0;
            while i < self.w {
                let cell = row[i];
                let mut j = i + 1;
                while j < self.w
                    && row[j].fg == cell.fg
                    && row[j].bold == cell.bold
                    && row[j].italic == cell.italic
                {
                    j += 1;
                }
                let run: String = row[i..j].iter().map(|c| c.ch).collect();
                let _ = queue!(out, SetForegroundColor(cell.fg));
                if cell.bold {
                    let _ = queue!(out, SetAttribute(Attribute::Bold));
                }
                if cell.italic {
                    let _ = queue!(out, SetAttribute(Attribute::Italic));
                }
                let _ = queue!(out, Print(run));
                if cell.bold || cell.italic {
                    let _ = queue!(out, SetAttribute(Attribute::Reset));
                }
                i = j;
            }
            let _ = queue!(out, ResetColor);
        }
        let _ = out.flush();
        self.prev.clone_from(&self.cur);
        changed
    }
}

// ===== UI state ==========================================================

#[derive(Default, Clone, Copy, PartialEq)]
enum Focus {
    #[default]
    Menu,
    Board,
}

struct Ui {
    focus: Focus,
    /// Inspection cursor over the four field rows (0..4) × five columns (0..5).
    row: usize,
    col: usize,
    /// Selected menu item, and the first visible menu row (scroll offset).
    sel: usize,
    menu_scroll: usize,
    /// The GY pile currently under the mouse (its owner), so it highlights and
    /// previews its top card. `None` when the mouse isn't over a pile.
    gy_preview: Option<usize>,
    screen: Screen,
}

impl Ui {
    fn new() -> Self {
        Ui {
            focus: Focus::default(),
            row: 0,
            col: 0,
            sel: 0,
            menu_scroll: 0,
            gy_preview: None,
            screen: Screen::new(),
        }
    }
}

/// A menu choice: send a response to the engine, or a local view.
enum Act {
    Respond(Vec<u8>),
    ViewGy(usize),
}

struct Item {
    label: String,
    act: Act,
    /// The card this item is *about*, if any — used to show its details when the
    /// item is highlighted, and to trigger it by selecting that card on the board.
    card: Option<CardId>,
}

/// An item with no associated card (e.g. "next phase", "view graveyard").
fn respond(label: impl Into<String>, bytes: Vec<u8>) -> Item {
    Item {
        label: label.into(),
        act: Act::Respond(bytes),
        card: None,
    }
}

/// An item that acts on `card` — hoverable for details, selectable on the board.
fn respond_card(label: impl Into<String>, bytes: Vec<u8>, card: CardId) -> Item {
    Item {
        label: label.into(),
        act: Act::Respond(bytes),
        card: Some(card),
    }
}

// ===== Card naming & flag decoding =======================================

const TYPE_MONSTER: u32 = 0x1;
const TYPE_NORMAL: u32 = 0x10;

/// Whether a card is a Monster (vs a Spell/Trap), from its harvested type.
fn is_monster(duel: &Duel, id: CardId) -> bool {
    duel.card_data(id)
        .is_some_and(|d| d.card_type & TYPE_MONSTER != 0)
}

/// The card's name, straight from its harvested record (`data.name`). Falls back
/// to "?" for a nameless card (shouldn't happen for cards the demo sets up).
fn name_of(duel: &Duel, id: CardId) -> &str {
    match duel.get_card(id) {
        Some(c) if !c.data.name.is_empty() => &c.data.name,
        _ => "?",
    }
}

fn pos_abbr(pos: Position) -> &'static str {
    match pos {
        Position::FaceUpAttack => "ATK",
        Position::FaceUpDefense => "DEF",
        Position::FaceDownDefense => "SET",
        Position::FaceDownAttack => "f.d.ATK",
    }
}

/// Each player's signature colour — stable per player index, so you can tell the
/// sides apart even as top/bottom swap between turns (hotseat).
fn player_color(player: usize) -> Color {
    if player == 0 {
        Color::Blue
    } else {
        Color::Magenta
    }
}

/// Border colour for a slot by the monster's position (empty → dim).
fn pos_color(slot: Option<Position>) -> Color {
    match slot {
        Some(Position::FaceUpAttack) => Color::Red,
        Some(Position::FaceUpDefense) => Color::Cyan,
        Some(Position::FaceDownDefense) => Color::Grey,
        Some(Position::FaceDownAttack) => Color::DarkYellow,
        None => Color::DarkGrey,
    }
}

// Card-face colours by type (the requested exact hues). On hover, a lighter shade.
const SPELL_RGB: Color = Color::Rgb {
    r: 0x28,
    g: 0x8E,
    b: 0x61,
};
const NORMAL_RGB: Color = Color::Rgb {
    r: 0xFF,
    g: 0xFD,
    b: 0xD0,
};
const EFFECT_RGB: Color = Color::Rgb {
    r: 0xC8,
    g: 0x62,
    b: 0x28,
};
// A distinct gold for the NAME of a card whose effect is activatable right now.
const ACTIONABLE_RGB: Color = Color::Rgb {
    r: 0xFF,
    g: 0xD2,
    b: 0x4A,
};

/// A card box's base colour, from its type: spell / normal monster / effect monster.
fn card_color(duel: &Duel, id: CardId) -> Color {
    let t = duel.card_data(id).map(|d| d.card_type).unwrap_or(0);
    if t & TYPE_MONSTER != 0 {
        if t & TYPE_NORMAL != 0 {
            NORMAL_RGB
        } else {
            EFFECT_RGB
        }
    } else {
        SPELL_RGB // spell (traps fall here too for now)
    }
}

/// Shift an RGB colour `f` of the way toward white — for the hover highlight. A
/// non-RGB colour passes through unchanged.
fn lighten(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => {
            let up = |v: u8| (v as f32 + (255.0 - v as f32) * f).round() as u8;
            Color::Rgb {
                r: up(r),
                g: up(g),
                b: up(b),
            }
        }
        other => other,
    }
}

fn type_desc(t: u32) -> String {
    if t == 0 {
        return "—".into();
    }
    let mut parts = Vec::new();
    for (bit, word) in [
        (0x10u32, "Normal"),
        (0x20, "Effect"),
        (0x40, "Fusion"),
        (0x80, "Ritual"),
        (0x2000, "Synchro"),
        (0x800000, "Xyz"),
        (0x1000000, "Pendulum"),
        (0x4000000, "Link"),
    ] {
        if t & bit != 0 {
            parts.push(word);
        }
    }
    parts.push(if t & 0x1 != 0 {
        "Monster"
    } else if t & 0x2 != 0 {
        "Spell"
    } else if t & 0x4 != 0 {
        "Trap"
    } else {
        "Card"
    });
    parts.join(" ")
}

fn attribute_name(a: u32) -> &'static str {
    match a {
        0x01 => "EARTH",
        0x02 => "WATER",
        0x04 => "FIRE",
        0x08 => "WIND",
        0x10 => "LIGHT",
        0x20 => "DARK",
        0x40 => "DIVINE",
        _ => "—",
    }
}

fn race_name(r: u64) -> &'static str {
    match r {
        0x1 => "Warrior",
        0x2 => "Spellcaster",
        0x4 => "Fairy",
        0x8 => "Fiend",
        0x10 => "Zombie",
        0x20 => "Machine",
        0x2000 => "Dragon",
        0x4000 => "Beast",
        0x8000 => "Beast-Warrior",
        _ => "—",
    }
}

// ===== Layout ============================================================

const CELL_W: usize = 9; // inner width of a card cell
const CELL_TW: u16 = CELL_W as u16 + 2; // with borders
const CELL_H: u16 = 4; // rows per cell: top border, name, stats, bottom border
const GAP: u16 = 1;
const LABEL_X: u16 = 1;
const BOARD_X: u16 = 5; // x of the first cell (row labels sit to the left)

// Vertical layout — compact (3-row cells) so the board leaves room for the menu
// on a normal ~24-row terminal.
const TITLE_Y: u16 = 0;
const OPP_HDR_Y: u16 = 1;
const OPP_HAND_Y: u16 = 2; // opponent hand (face-down)
const R0_Y: u16 = OPP_HAND_Y + CELL_H; // opponent S/T
const R1_Y: u16 = R0_Y + CELL_H; // opponent Monsters
const DIV_Y: u16 = R1_Y + CELL_H; // centre divider
const R2_Y: u16 = DIV_Y + 1; // your Monsters
const R3_Y: u16 = R2_Y + CELL_H; // your S/T
const YOUR_HAND_Y: u16 = R3_Y + CELL_H; // your hand (face-up)
const YOU_HDR_Y: u16 = YOUR_HAND_Y + CELL_H;
const MENU_Y: u16 = YOU_HDR_Y + 2;

// The GY pile column (a 6th column, right of the 5 field cells), then the info
// panel just past it.
const PILE_X: u16 = BOARD_X + 5 * (CELL_TW + GAP); // = cell_x(5)
const BOARD_RIGHT: u16 = PILE_X + CELL_TW; // x just past the board (incl. piles)
const PANEL_X: u16 = BOARD_RIGHT + 2;

// The always-visible "Next Phase" button, up in the title row (the bottom menu can
// scroll off short terminals — this never does). Greyed while it isn't a legal move
// (e.g. responding to a chain).
const NEXT_LABEL: &str = "[ Next Phase ]";
const NEXT_X: u16 = 48;
const NEXT_Y: u16 = TITLE_Y;

/// The response bytes of the current "next phase / end phase" menu option, if one
/// is available right now (i.e. the button is live).
fn next_phase_bytes(items: &[Item]) -> Option<Vec<u8>> {
    items.iter().find_map(|it| match &it.act {
        Act::Respond(b) if b.first() == Some(&CMD_NEXT_PHASE) => Some(b.clone()),
        _ => None,
    })
}

/// Whether a click at `(col, row)` lands on the Next Phase button.
fn next_hit(col: u16, row: u16) -> bool {
    row == NEXT_Y && (NEXT_X..NEXT_X + NEXT_LABEL.len() as u16).contains(&col)
}

/// Rows the chain-response panel occupies at the bottom of the viewport: a title
/// line plus one row of card-sized boxes (`CELL_H` tall).
const CHAIN_PANEL_H: u16 = CELL_H + 1;

/// Prompts whose menu we overlay as clickable boxes at the bottom of the *visible*
/// screen instead of the off-screen menu: a chain-response window, or a yes/no
/// (e.g. an optional trigger like Optional Avenger). Otherwise the tall board can
/// push the real menu below the fold where it can't be seen or clicked.
fn wants_bottom_panel(duel: &Duel) -> bool {
    matches!(
        duel.messages().last(),
        Some(&MSG_SELECT_CHAIN) | Some(&MSG_SELECT_YESNO)
    )
}

/// The label shown inside a chain-panel box for menu item `it` — "PASS" for the
/// pass option, otherwise the chainable card's name.
fn chain_box_label(it: &Item) -> &str {
    match &it.act {
        Act::Respond(b) if b.first() == Some(&CMD_PASS) => "PASS",
        _ => it.label.strip_prefix("Chain: ").unwrap_or(&it.label),
    }
}

/// Lay the chain-response choices out as card-sized boxes along the bottom of the
/// viewport — a PASS box plus one per chainable card. Returns `(item_index, x, y,
/// width)` each. Whole-box hit areas make them easy to click (the old thin one-row
/// button was nearly impossible to hit).
fn chain_boxes(items: &[Item], screen_w: usize, screen_h: usize) -> Vec<(usize, u16, u16, u16)> {
    let y = (screen_h as u16).saturating_sub(CELL_H);
    let mut out = Vec::new();
    let mut x = BOARD_X;
    for i in 0..items.len() {
        let w = CELL_TW;
        if (x + w) as usize >= screen_w {
            break; // don't run off the right edge
        }
        out.push((i, x, y, w));
        x += w + GAP;
    }
    out
}

/// The chain-panel box under `(col, row)`, as a menu-item index — for both clicking
/// and hovering. `None` if not a chain prompt or the point missed every box.
fn chain_box_at(duel: &Duel, ui: &Ui, items: &[Item], col: u16, row: u16) -> Option<usize> {
    if !wants_bottom_panel(duel) {
        return None;
    }
    chain_boxes(items, ui.screen.w, ui.screen.h)
        .into_iter()
        .find(|&(_, x, y, w)| (y..y + CELL_H).contains(&row) && (x..x + w).contains(&col))
        .map(|(i, ..)| i)
}

/// A click anywhere on a chain-panel box → that option's response bytes.
fn chain_panel_click(duel: &Duel, ui: &Ui, items: &[Item], col: u16, row: u16) -> Option<Vec<u8>> {
    let i = chain_box_at(duel, ui, items, col, row)?;
    match &items[i].act {
        Act::Respond(bytes) => Some(bytes.clone()),
        Act::ViewGy(_) => None,
    }
}

/// Hovering a chain-panel box → its menu-item index (to highlight it).
fn chain_hover(duel: &Duel, ui: &Ui, items: &[Item], col: u16, row: u16) -> Option<usize> {
    chain_box_at(duel, ui, items, col, row)
}

/// Width of the right-hand panel for a terminal `screen_w` cols wide — it shrinks
/// to fit and is capped at 24.
fn panel_w(screen_w: usize) -> usize {
    // Reserve the panel's left+right borders so it never spills off the edge.
    (screen_w as u16).saturating_sub(PANEL_X + 2).clamp(8, 24) as usize
}

/// Rows the always-on status panel occupies at the bottom of the right column.
const STATUS_H: u16 = 4;

/// A small, always-visible status panel anchored to the bottom of the right column
/// (below the card-details panel): turn + phase, and both players' LP — so this
/// core info is on screen no matter how short the terminal or where the off-screen
/// player headers sit. `you` is the viewer (their LP is labelled "You").
fn draw_status_panel(scr: &mut Screen, duel: &Duel, you: usize) {
    if !has_side_panel(scr.w) {
        return; // no right column to put it in on a narrow terminal
    }
    let opp = 1 - you;
    let x = PANEL_X;
    let w = panel_w(scr.w);
    let top = (scr.h as u16).saturating_sub(STATUS_H);

    // A labelled separator so it reads as its own panel.
    let dashes = w.saturating_sub(8);
    scr.put(
        x,
        top,
        &format!("── Status {}", "─".repeat(dashes)),
        Color::DarkGrey,
    );
    scr.put_bold(
        x,
        top + 1,
        &format!(
            "Turn {} · {}",
            duel.turn_history().len(),
            current_phase_label(duel)
        ),
        Color::White,
    );
    for (i, (label, p)) in [("You", you), ("Opp", opp)].into_iter().enumerate() {
        let y = top + 2 + i as u16;
        let lp = duel.life_points(p);
        let lp_color = if lp <= 2000 { Color::Red } else { Color::Green };
        scr.put(x, y, &format!("{label} (P{p})  LP"), player_color(p));
        scr.put_bold(x + 12, y, &format!("{lp:>5}"), lp_color);
    }
}

/// Is the terminal wide enough to show the right-hand column at all?
fn has_side_panel(screen_w: usize) -> bool {
    screen_w as u16 >= PANEL_X + 10
}

/// The six navigable rows, from `you`'s view (top → bottom): opponent hand, S/T,
/// Monsters; then your Monsters, S/T, hand. (The opponent's hand is face-down.)
fn nav_rows(you: usize) -> [(usize, Zone, u16, &'static str); 6] {
    let opp = 1 - you;
    [
        (opp, Zone::Hand, OPP_HAND_Y, "Hand"),
        (opp, Zone::SpellTrapZone, R0_Y, "S/T"),
        (opp, Zone::MonsterZone, R1_Y, "Mon"),
        (you, Zone::MonsterZone, R2_Y, "Mon"),
        (you, Zone::SpellTrapZone, R3_Y, "S/T"),
        (you, Zone::Hand, YOUR_HAND_Y, "Hand"),
    ]
}

fn zone_cards(duel: &Duel, player: usize, zone: Zone) -> Vec<CardId> {
    match zone {
        Zone::MonsterZone => duel.monster_zone(player),
        Zone::SpellTrapZone => duel.spell_trap_zone(player),
        Zone::GY => duel.graveyard(player),
        Zone::Hand => (0..duel.hand_count(player))
            .filter_map(|i| duel.hand_card(player, i))
            .collect(),
        _ => Vec::new(),
    }
}

fn cell_x(col: usize) -> u16 {
    BOARD_X + col as u16 * (CELL_TW + GAP)
}

/// Whether a `(player, zone)` is hidden from `you` — only the opponent's hand.
fn is_hidden(player: usize, zone: Zone, you: usize) -> bool {
    zone == Zone::Hand && player != you
}

/// The card the inspection cursor is currently over, if any. The opponent's hand
/// is face-down, so it never reveals a card.
fn card_under_cursor(duel: &Duel, ui: &Ui, you: usize) -> Option<CardId> {
    let (player, zone, _, _) = nav_rows(you)[ui.row];
    if is_hidden(player, zone, you) {
        return None;
    }
    zone_cards(duel, player, zone).get(ui.col).copied()
}

// ===== Rendering (into the Screen buffer) ================================

/// Pad/truncate a string to exactly `w` chars.
fn fit(s: &str, w: usize) -> String {
    let t: String = s.chars().take(w).collect();
    let pad = w - t.chars().count();
    format!("{t}{}", " ".repeat(pad))
}

fn draw_cell(
    scr: &mut Screen,
    x: u16,
    y: u16,
    duel: &Duel,
    slot: Option<CardId>,
    hi: bool,
    actionable: bool,
) {
    // The box is coloured by card type (spell / normal / effect monster), a lighter
    // shade on hover. An empty slot is dim.
    let base = match slot {
        Some(id) => card_color(duel, id),
        None => Color::DarkGrey,
    };
    let box_color = if hi { lighten(base, 0.4) } else { base };
    let box_bold = hi;
    // Actionable → only the NAME text glows a distinct gold (the box stays its type
    // colour), so you can see at a glance which cards you can act on.
    let name_base = if actionable { ACTIONABLE_RGB } else { base };
    let name_color = if hi {
        lighten(name_base, 0.4)
    } else {
        name_base
    };
    let name_bold = hi || actionable;

    let bar = "─".repeat(CELL_W);
    let (name, stats) = match slot {
        Some(id) => {
            let stats = match (duel.atk_of(id), duel.def_of(id)) {
                (Some(a), Some(d)) => format!("{a}/{d}"),
                _ => String::new(),
            };
            (fit(name_of(duel, id), CELL_W), fit(&stats, CELL_W))
        }
        None => (fit("", CELL_W), fit("    ·", CELL_W)),
    };

    scr.put_styled(x, y, &format!("┌{bar}┐"), box_color, box_bold, false);
    // Name row: the side borders keep the box colour; only the name text is tinted.
    scr.put_styled(x, y + 1, "│", box_color, box_bold, false);
    scr.put_styled(x + 1, y + 1, &name, name_color, name_bold, false);
    scr.put_styled(
        x + 1 + CELL_W as u16,
        y + 1,
        "│",
        box_color,
        box_bold,
        false,
    );
    scr.put_styled(x, y + 2, &format!("│{stats}│"), box_color, box_bold, false);
    scr.put_styled(x, y + 3, &format!("└{bar}┘"), box_color, box_bold, false);
}

fn draw_header(scr: &mut Screen, y: u16, duel: &Duel, player: usize, label: &str) {
    let lp = duel.life_points(player);
    let lp_color = if lp <= 2000 { Color::Red } else { Color::Green };
    // The player's name label in their signature colour (LP keeps its danger hue).
    // Pad to a fixed width so the columns after it stay aligned for both players.
    scr.put_bold(
        LABEL_X,
        y,
        &format!("{:<14}", format!("{label} (P{player})")),
        player_color(player),
    );
    scr.put(LABEL_X + 15, y, "LP ", Color::Grey);
    scr.put_bold(LABEL_X + 18, y, &format!("{lp:>5}"), lp_color);
    scr.put(
        LABEL_X + 25,
        y,
        &format!(
            "Deck {}   Hand {}   GY {}",
            duel.deck_count(player),
            duel.hand_count(player),
            duel.graveyard(player).len(),
        ),
        Color::Grey,
    );
}

/// Draw the whole left board: title bar, both player headers, and the four field
/// rows (opponent on top, you on the bottom). Cursor highlight only in board
/// focus. Shared by the normal view and the graveyard inspector.
fn draw_board(
    scr: &mut Screen,
    duel: &Duel,
    focus: Focus,
    cursor: (usize, usize),
    you: usize,
    actionable: &[CardId],
    gy_preview: Option<usize>,
) {
    let (row, col) = cursor;
    scr.put_bold(
        LABEL_X,
        TITLE_Y,
        &format!(
            "Turn {} — Player {you} — {}",
            duel.turn_history().len(),
            current_phase_label(duel),
        ),
        Color::White,
    );
    let (focus_txt, focus_col) = match focus {
        Focus::Menu => ("[MENU]", Color::Cyan),
        Focus::Board => ("[BOARD]", Color::Yellow),
    };
    scr.put(PANEL_X, TITLE_Y, focus_txt, focus_col);

    let opp = 1 - you;
    draw_header(scr, OPP_HDR_Y, duel, opp, "Opponent");
    for (ri, (player, zone, y, label)) in nav_rows(you).into_iter().enumerate() {
        // Row label in the controlling player's signature colour.
        scr.put(LABEL_X, y + 1, label, player_color(player));
        let hidden = is_hidden(player, zone, you);
        let count = zone_cards(duel, player, zone).len();
        let cards = zone_cards(duel, player, zone);
        for c in 0..5 {
            let hi = focus == Focus::Board && row == ri && col == c;
            if hidden {
                // Opponent's hand: face-down backs, one per card they hold.
                draw_back(scr, cell_x(c), y, c < count, hi);
            } else {
                let slot = cards.get(c).copied();
                let act = slot.is_some_and(|id| actionable.contains(&id));
                draw_cell(scr, cell_x(c), y, duel, slot, hi, act);
            }
        }
        if y == R1_Y {
            scr.put(
                LABEL_X,
                DIV_Y,
                &"─".repeat(BOARD_RIGHT as usize),
                Color::DarkGrey,
            );
        }
    }
    // GY piles in the 6th column (hover to preview its top card, right-click to
    // open the full list — see `pile_hit`).
    draw_pile(
        scr,
        PILE_X,
        R0_Y,
        "GY",
        duel.graveyard(opp).len(),
        gy_preview == Some(opp),
    );
    draw_pile(
        scr,
        PILE_X,
        R3_Y,
        "GY",
        duel.graveyard(you).len(),
        gy_preview == Some(you),
    );
    draw_header(scr, YOU_HDR_Y, duel, you, "You");
}

/// A face-down card cell (opponent's hand). `present` = an actual card sits here.
fn draw_back(scr: &mut Screen, x: u16, y: u16, present: bool, hi: bool) {
    let color = if hi { Color::Yellow } else { Color::DarkGrey };
    let bar = "─".repeat(CELL_W);
    let body = if present {
        "  ▚▚▚▚▚"
    } else {
        "    ·"
    };
    scr.put(x, y, &format!("┌{bar}┐"), color);
    scr.put(x, y + 1, &format!("│{}│", fit(body, CELL_W)), color);
    scr.put(x, y + 2, &format!("│{}│", fit(body, CELL_W)), color);
    scr.put(x, y + 3, &format!("└{bar}┘"), color);
}

/// A pile box (GY): a labelled box showing its card count. `hi` = under the mouse.
fn draw_pile(scr: &mut Screen, x: u16, y: u16, label: &str, count: usize, hi: bool) {
    let color = if hi { Color::White } else { Color::Grey };
    let bar = "─".repeat(CELL_W);
    scr.put(x, y, &format!("┌{bar}┐"), color);
    scr.put(x, y + 1, &format!("│{}│", fit(label, CELL_W)), color);
    scr.put(
        x,
        y + 2,
        &format!("│{}│", fit(&format!("{count} card"), CELL_W)),
        color,
    );
    scr.put(x, y + 3, &format!("└{bar}┘"), color);
}

/// If a click at `(col, row)` lands on a GY pile box, return whose GY it is.
fn pile_hit(you: usize, col: u16, row: u16) -> Option<usize> {
    if !(PILE_X..PILE_X + CELL_TW).contains(&col) {
        return None;
    }
    if (R0_Y..R0_Y + CELL_H).contains(&row) {
        Some(1 - you) // opponent's GY (top)
    } else if (R3_Y..R3_Y + CELL_H).contains(&row) {
        Some(you) // your GY (bottom)
    } else {
        None
    }
}

/// Draw the board + menu. Returns whether the frame changed since the last one.
fn render(duel: &Duel, ui: &mut Ui, title: &str, items: &[Item]) -> bool {
    let you = current(duel);
    let focused = focused_card(duel, ui, items);

    // Update the menu's scroll offset (edge-triggered) before borrowing the
    // screen. The window is the rows between the menu header and the last line.
    let menu_visible = visible_rows(ui.screen.h, MENU_Y + 1);
    ui.menu_scroll = scroll_offset(ui.sel, items.len(), menu_visible, ui.menu_scroll);

    // Cards you can act on right now (an attacker, a target, a summon, an
    // activatable/chainable effect) — every `Respond` item that's about a card.
    // They glow on the board so on-board clicking is discoverable.
    let actionable: Vec<CardId> = items
        .iter()
        .filter_map(|it| match &it.act {
            Act::Respond(_) => it.card,
            _ => None,
        })
        .collect();

    let (focus, row, col, sel, offset, gy_preview) = (
        ui.focus,
        ui.row,
        ui.col,
        ui.sel,
        ui.menu_scroll,
        ui.gy_preview,
    );
    let scr = &mut ui.screen;
    scr.begin();

    draw_board(scr, duel, focus, (row, col), you, &actionable, gy_preview);

    // The Next Phase button — green when live, grey when it isn't a legal move.
    let np_live = next_phase_bytes(items).is_some();
    scr.put_styled(
        NEXT_X,
        NEXT_Y,
        NEXT_LABEL,
        if np_live {
            Color::Green
        } else {
            Color::DarkGrey
        },
        np_live,
        false,
    );

    // Menu (bottom), over the field — a scrolling viewport that follows the
    // selection, so a long list can't run off the screen. The window shows
    // `visible` options; moving past the bottom scrolls, hiding the top.
    let menu_dim = focus == Focus::Board;
    let first_row = MENU_Y + 1;
    let visible = menu_visible;
    let end = (offset + visible).min(items.len());

    let mut header = format!("▸ {title}");
    if items.len() > visible {
        header.push_str(&format!("   [{}-{} of {}]", offset + 1, end, items.len()));
    }
    scr.put_bold(
        LABEL_X,
        MENU_Y,
        &header,
        if menu_dim {
            Color::DarkGrey
        } else {
            Color::White
        },
    );

    for (row, i) in (offset..end).enumerate() {
        let y = first_row + row as u16;
        let (marker, color) = match (i == sel, menu_dim) {
            (true, false) => ("›", Color::Yellow),
            (true, true) => ("·", Color::Grey),
            _ => (" ", Color::White),
        };
        // Edge rows hint at more items above/below when the list is clipped.
        let marker = if row == 0 && offset > 0 {
            "↑"
        } else if row + 1 == (end - offset) && end < items.len() {
            "↓"
        } else {
            marker
        };
        scr.put(LABEL_X, y, &format!("{marker} {}", items[i].label), color);
    }
    scr.put(
        LABEL_X,
        first_row + (end - offset) as u16,
        "Click select · Hover to preview · scroll move · q quit",
        Color::DarkGrey,
    );

    // Right column: the hovered/selected card's details (the hand is now a row on
    // the field). Skipped entirely on a terminal too narrow to hold it.
    if has_side_panel(scr.w) {
        if let Some(id) = focused {
            draw_card_panel(scr, id, duel, R0_Y);
        }
    }

    // Always-on status panel at the bottom of the right column (turn/phase + both
    // LPs). Drawn after the card panel so it stays visible even if that runs long.
    draw_status_panel(scr, duel, you);

    // Chain-response panel: card-sized boxes at the bottom of the VISIBLE screen
    // (never scrolls off), overlaid on the field — a PASS box plus one per chainable
    // card. The whole box is clickable.
    if wants_bottom_panel(duel) {
        let top = (scr.h as u16).saturating_sub(CHAIN_PANEL_H);
        // Blank the panel band (over the board's bottom rows) so nothing bleeds
        // through, then draw a header line + the boxes on top.
        for yy in top..scr.h as u16 {
            scr.put(0, yy, &" ".repeat(BOARD_RIGHT as usize), Color::Reset);
        }
        scr.put_bold(
            LABEL_X,
            top,
            &format!("▸ {title}   (click a box)"),
            Color::White,
        );
        for (i, x, y, _w) in chain_boxes(items, scr.w, scr.h) {
            // A "decline" box (Pass, or No = response byte 0) reads grey; an
            // affirmative choice (a chainable card, or Yes) reads yellow.
            let declines = matches!(&items[i].act,
                Act::Respond(b) if b.first() == Some(&CMD_PASS) || b.as_slice() == [0]);
            let hot = i == sel;
            let color = if hot {
                Color::White
            } else if declines {
                Color::Grey
            } else {
                Color::Yellow
            };
            let label = chain_box_label(&items[i]);
            let bar = "─".repeat(CELL_W);
            scr.put_styled(x, y, &format!("┌{bar}┐"), color, true, false);
            scr.put_styled(
                x,
                y + 1,
                &format!("│{}│", fit(label, CELL_W)),
                color,
                true,
                false,
            );
            scr.put_styled(
                x,
                y + 2,
                &format!("│{}│", fit("", CELL_W)),
                color,
                true,
                false,
            );
            scr.put_styled(x, y + 3, &format!("└{bar}┘"), color, true, false);
        }
    }

    scr.flush(&mut io::stdout())
}

/// The card whose details the right panel should show: a hovered GY pile's top
/// card takes precedence, then the slot under the inspection cursor (board focus),
/// then the card the selected menu item is about (menu focus). `None` → no panel.
fn focused_card(duel: &Duel, ui: &Ui, items: &[Item]) -> Option<CardId> {
    if let Some(p) = ui.gy_preview {
        return duel.graveyard(p).last().copied(); // most-recent card in that GY
    }
    match ui.focus {
        Focus::Board => card_under_cursor(duel, ui, current(duel)),
        Focus::Menu => items.get(ui.sel).and_then(|it| it.card),
    }
}

/// Draw the card-details panel at the top of the right column. Returns the `y`
/// of its bottom border (so the hand panel can sit right under it). Spells and
/// monsters get different layouts (a spell has no ATK/DEF/level — just its text).
fn draw_card_panel(scr: &mut Screen, id: CardId, duel: &Duel, top_y: u16) -> u16 {
    let data = match duel.card_data(id) {
        Some(d) => d,
        None => return R0_Y.saturating_sub(1),
    };

    let is_monster = data.card_type & TYPE_MONSTER != 0;
    let is_normal = data.card_type & TYPE_NORMAL != 0;

    // Name + type header, shared by both layouts.
    let mut lines: Vec<PanelLine> = vec![
        (name_of(duel, id).to_string(), Color::White, false),
        (type_desc(data.card_type), Color::Grey, false),
    ];

    let (title, body_color) = if !is_monster {
        // Spell layout: its effect text, and nothing else. (Normal Spells only.)
        (" Spell ", Color::White)
    } else {
        // Monster layout: the battle numbers.
        lines.push((
            format!("ATK {}   DEF {}", data.atk, data.def),
            Color::Green,
            false,
        ));
        lines.push((
            format!("Level {}", data.level.unwrap_or(0)),
            Color::Grey,
            false,
        ));
        lines.push((
            format!(
                "{} · {}",
                attribute_name(data.attribute),
                race_name(data.race)
            ),
            Color::Cyan,
            false,
        ));
        if let Some(pos) = duel.position_of(id) {
            lines.push((
                format!("Position: {}", pos_abbr(pos)),
                pos_color(Some(pos)),
                false,
            ));
        }
        (" Monster ", Color::DarkGrey)
    };

    // Card text: flavour text of a Normal Monster is shown in italics; a Spell's
    // (or effect) text is upright.
    let text_italic = is_monster && is_normal;
    if data.text.is_empty() {
        if !is_monster {
            lines.push(("(no text)".into(), Color::DarkGrey, false));
        }
    } else {
        lines.push((String::new(), Color::Grey, false));
        for chunk in wrap(&data.text, panel_w(scr.w)) {
            lines.push((chunk, body_color, text_italic));
        }
    }

    draw_panel(scr, top_y, title, &lines, 0)
}

/// Draw a titled box at `(PANEL_X, top_y)` with the given lines. Returns the `y`
/// of the bottom border.
/// One line in a panel: its text, colour, and whether it's italic.
type PanelLine = (String, Color, bool);

/// How many content rows fit below a header row at `top_y` while keeping the
/// bottom border (or hint line) on-screen.
fn visible_rows(h: usize, top_y: u16) -> usize {
    (h as u16).saturating_sub(top_y + 2).max(1) as usize
}

/// Edge-triggered scroll: nudge `offset` *minimally* so `sel` stays within a
/// `visible`-row window over `len` items. The highlight moves freely inside the
/// window and the list only scrolls when the highlight reaches an edge.
fn scroll_offset(sel: usize, len: usize, visible: usize, offset: usize) -> usize {
    if visible == 0 || len <= visible {
        return 0;
    }
    let max_off = len - visible;
    let mut off = offset.min(max_off);
    if sel < off {
        off = sel; // scrolled above the window → pull up to it
    } else if sel >= off + visible {
        off = sel + 1 - visible; // below the window → pull down to it
    }
    off
}

/// Draw a titled box at `(PANEL_X, top_y)` with `lines`, **clipped to the screen
/// height** so the bottom border stays on-screen. `offset` is the first visible
/// line (the caller owns the scroll state — see `scroll_offset`); pass `0` to
/// clip from the top. A `[a-b of n]` counter appears in the title when clipped.
/// Returns the `y` of the bottom border.
fn draw_panel(
    scr: &mut Screen,
    top_y: u16,
    title: &str,
    lines: &[PanelLine],
    offset: usize,
) -> u16 {
    let max_rows = visible_rows(scr.h, top_y);
    let n = lines.len();
    let offset = offset.min(n.saturating_sub(max_rows)); // never scroll past the end
    let end = (offset + max_rows).min(n);

    let pw = panel_w(scr.w);
    let bar = "─".repeat(pw);
    scr.put(PANEL_X, top_y, &format!("┌{bar}┐"), Color::DarkGrey);
    let mut header = title.to_string();
    if n > max_rows {
        header.push_str(&format!("[{}-{} of {}]", offset + 1, end, n));
    }
    scr.put(PANEL_X + 2, top_y, &header, Color::Grey); // title overlays the border

    for (row, i) in (offset..end).enumerate() {
        let y = top_y + 1 + row as u16;
        let (text, color, italic) = &lines[i];
        scr.put(PANEL_X, y, "│", Color::DarkGrey);
        scr.put_styled(PANEL_X + 1, y, &fit(text, pw), *color, false, *italic);
        scr.put(PANEL_X + 1 + pw as u16, y, "│", Color::DarkGrey);
    }
    let bottom = top_y + 1 + (end - offset) as u16;
    scr.put(PANEL_X, bottom, &format!("└{bar}┘"), Color::DarkGrey);
    bottom
}

/// Wrap `s` into lines of at most `w` chars (whitespace-aware, best effort).
fn wrap(s: &str, w: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if line.chars().count() + word.chars().count() + 1 > w && !line.is_empty() {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn hand_label(duel: &Duel, id: CardId) -> String {
    let name = name_of(duel, id);
    match (is_monster(duel, id), duel.atk_of(id), duel.def_of(id)) {
        (true, Some(a), Some(d)) => format!("{name} ({a}/{d})"),
        _ => name.to_string(),
    }
}

fn current_phase_label(duel: &Duel) -> &'static str {
    duel.messages()
        .iter()
        .rev()
        .find_map(|&m| phase_label(m))
        .unwrap_or("…")
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
        _ => return None,
    })
}

// ===== Input loop ========================================================

/// If a click at `(col, row)` lands on a visible menu item, return its index.
/// The menu occupies the left region below the board (`col < PANEL_X`).
fn menu_hit(ui: &Ui, item_count: usize, col: u16, row: u16) -> Option<usize> {
    let first_row = MENU_Y + 1;
    if col >= PANEL_X || row < first_row {
        return None;
    }
    let shown = visible_rows(ui.screen.h, first_row).min(item_count.saturating_sub(ui.menu_scroll));
    let r = (row - first_row) as usize;
    (r < shown).then_some(ui.menu_scroll + r)
}

/// If a click lands on a board cell, return `(nav-row, column)` — the same
/// coordinates as the inspection cursor.
fn board_hit(you: usize, col: u16, row: u16) -> Option<(usize, usize)> {
    for (ri, (_, _, y, _)) in nav_rows(you).into_iter().enumerate() {
        if row >= y && row < y + CELL_H {
            for c in 0..5u16 {
                let x = cell_x(c as usize);
                if col >= x && col < x + CELL_TW {
                    return Some((ri, c as usize));
                }
            }
        }
    }
    None
}

/// Render the board + menu and drive input until the player picks a real action.
/// Returns the response bytes to hand to the engine. Mouse: click a menu item to
/// pick it, click a card to select/inspect it, scroll to move the highlight.
/// Keyboard still works too — Tab toggles board/menu focus.
fn run_menu(duel: &mut Duel, ui: &mut Ui, title: &str, items: &[Item]) -> Vec<u8> {
    if items.is_empty() {
        return vec![0];
    }
    ui.sel = ui.sel.min(items.len() - 1);
    loop {
        render(duel, ui, title, items);
        match read_input() {
            Input::Key(KeyCode::Tab) => {
                ui.focus = match ui.focus {
                    Focus::Menu => Focus::Board,
                    Focus::Board => Focus::Menu,
                }
            }
            Input::Key(code) => match ui.focus {
                Focus::Menu => match code {
                    KeyCode::Up => ui.sel = ui.sel.saturating_sub(1),
                    KeyCode::Down => ui.sel = (ui.sel + 1).min(items.len() - 1),
                    KeyCode::Enter => match &items[ui.sel].act {
                        Act::Respond(bytes) => return bytes.clone(),
                        Act::ViewGy(p) => view_gy(duel, ui, *p),
                    },
                    _ => {}
                },
                Focus::Board => match code {
                    KeyCode::Up => ui.row = ui.row.saturating_sub(1),
                    KeyCode::Down => ui.row = (ui.row + 1).min(5),
                    KeyCode::Left => ui.col = ui.col.saturating_sub(1),
                    KeyCode::Right => ui.col = (ui.col + 1).min(4),
                    // Enter on a slot: if a menu item is about that card (e.g. an
                    // attacker or an attack target), pick it right from the board.
                    KeyCode::Enter => {
                        let picked = card_under_cursor(duel, ui, current(duel))
                            .and_then(|card| item_for_card(items, card));
                        match picked {
                            Some(bytes) => return bytes,
                            None => ui.focus = Focus::Menu, // nothing to do here
                        }
                    }
                    KeyCode::Esc => ui.focus = Focus::Menu,
                    _ => {}
                },
            },
            // Scroll moves the menu highlight (like ↑/↓).
            Input::Scroll(d) => {
                ui.focus = Focus::Menu;
                ui.sel = if d < 0 {
                    ui.sel.saturating_sub(1)
                } else {
                    (ui.sel + 1).min(items.len() - 1)
                };
            }
            Input::Click(col, row) => {
                // The chain panel (bottom of the viewport) takes clicks first, then
                // the Next Phase button, then the menu / board.
                if let Some(bytes) = chain_panel_click(duel, ui, items, col, row) {
                    return bytes;
                } else if next_hit(col, row) {
                    if let Some(bytes) = next_phase_bytes(items) {
                        return bytes; // greyed → no bytes → ignored
                    }
                } else if let Some(i) = menu_hit(ui, items.len(), col, row) {
                    // A single click on a menu item selects AND activates it.
                    ui.sel = i;
                    ui.focus = Focus::Menu;
                    match &items[i].act {
                        Act::Respond(bytes) => return bytes.clone(),
                        Act::ViewGy(p) => view_gy(duel, ui, *p),
                    }
                } else if let Some((r, c)) = board_hit(current(duel), col, row) {
                    ui.focus = Focus::Board;
                    ui.row = r;
                    ui.col = c;
                    // Clicking a card that a menu item is about (attacker / target)
                    // picks it straight from the board.
                    if let Some(bytes) = card_under_cursor(duel, ui, current(duel))
                        .and_then(|card| item_for_card(items, card))
                    {
                        return bytes;
                    }
                }
            }
            // Hover PREVIEWS: move the selection/cursor onto the card under the
            // mouse so the details panel shows it — never select/act. Over empty
            // space, leave the current preview as-is.
            Input::Hover(col, row) => {
                // A GY pile is only previewed while the mouse is actually over it.
                ui.gy_preview = None;
                // Hovering a chain box highlights it (white) so it's clear it's the
                // click target.
                if let Some(i) = chain_hover(duel, ui, items, col, row) {
                    ui.sel = i;
                    ui.focus = Focus::Menu;
                } else if let Some(p) = pile_hit(current(duel), col, row) {
                    // Hover a GY pile → highlight it + preview its top card.
                    ui.gy_preview = Some(p);
                } else if let Some(i) = menu_hit(ui, items.len(), col, row) {
                    ui.sel = i;
                    ui.focus = Focus::Menu;
                } else if let Some((r, c)) = board_hit(current(duel), col, row) {
                    ui.focus = Focus::Board;
                    ui.row = r;
                    ui.col = c;
                }
            }
            // Right-click a GY pile → open its full card list.
            Input::RightClick(col, row) => {
                if let Some(p) = pile_hit(current(duel), col, row) {
                    view_gy(duel, ui, p);
                }
            }
        }
    }
}

/// The response bytes of the menu item that acts on `card`, if any — the bridge
/// from "a card selected on the board" to "the matching menu action".
fn item_for_card(items: &[Item], card: CardId) -> Option<Vec<u8>> {
    items.iter().find_map(|it| match (&it.act, it.card) {
        (Act::Respond(bytes), Some(c)) if c == card => Some(bytes.clone()),
        _ => None,
    })
}

/// The two graveyards, offered as inspectable public zones in idle menus.
fn view_items(you: usize) -> Vec<Item> {
    let opp = 1 - you;
    vec![
        Item {
            label: "View your Graveyard".into(),
            act: Act::ViewGy(you),
            card: None,
        },
        Item {
            label: "View opponent's Graveyard".into(),
            act: Act::ViewGy(opp),
            card: None,
        },
    ]
}

/// A yes/no prompt — currently only an optional trigger asking its controller
/// whether to activate. Response `[1]` = yes, `[0]` = no.
fn yesno_menu(duel: &mut Duel, ui: &mut Ui) {
    let items = vec![respond("Yes", vec![1]), respond("No", vec![0])];
    let bytes = run_menu(duel, ui, "Activate the optional effect?", &items);
    duel.set_response(&bytes);
}

/// A chain response window: the current responder may chain a fast effect onto the
/// top link, or pass. `[CMD_PASS]` passes; `[CMD_RESPONSE, i]` chains option `i`.
fn chain_response_menu(duel: &mut Duel, ui: &mut Ui) {
    let responder = duel.chain_responder();
    let mut items = vec![respond("Pass — no response", vec![CMD_PASS])];
    for (opt, (card, _slot)) in duel.chain_response_options().into_iter().enumerate() {
        items.push(respond_card(
            format!("Chain: {}", name_of(duel, card)),
            vec![CMD_RESPONSE, opt as u8],
            card,
        ));
    }
    let title = format!("Player {responder}: respond to the chain?");
    let bytes = run_menu(duel, ui, &title, &items);
    duel.set_response(&bytes);
}

fn main_phase_menu(duel: &mut Duel, ui: &mut Ui) {
    let you = current(duel);
    let mut items = vec![respond("Go to next phase", vec![CMD_NEXT_PHASE])];

    // Only offer summons while the once-per-turn Normal Summon is still available
    // — after using it, these options can't do anything, so hide them.
    if duel.can_normal_summon(you) {
        for i in 0..duel.hand_count(you) {
            if let Some(id) = duel.hand_card(you, i) {
                if is_monster(duel, id) {
                    items.push(respond_card(
                        format!("Summon {}", name_of(duel, id)),
                        vec![CMD_SUMMON, i as u8],
                        id,
                    ));
                }
            }
        }
    }
    for (opt, (card, _slot)) in duel.activatable_effects(you).into_iter().enumerate() {
        items.push(respond_card(
            format!("Activate {}", name_of(duel, card)),
            vec![CMD_ACTIVATE, opt as u8],
            card,
        ));
    }
    items.extend(view_items(you));

    let bytes = run_menu(duel, ui, "Main Phase", &items);
    duel.set_response(&bytes);
}

fn battle_phase_menu(duel: &mut Duel, ui: &mut Ui) {
    let you = current(duel);
    let mut items = vec![respond("End Battle Phase", vec![CMD_NEXT_PHASE])];
    for (i, atk) in duel.attackers(you).into_iter().enumerate() {
        items.push(respond_card(
            format!("Attack with {}", name_of(duel, atk)),
            vec![CMD_ATTACK, i as u8],
            atk,
        ));
    }
    items.extend(view_items(you));

    let bytes = run_menu(duel, ui, "Battle Phase", &items);
    duel.set_response(&bytes);
}

fn select_attack_target(duel: &mut Duel, ui: &mut Ui) {
    let you = current(duel);
    let items: Vec<Item> = duel
        .attack_targets(you)
        .into_iter()
        .enumerate()
        .map(|(i, id)| respond_card(name_of(duel, id), vec![i as u8], id))
        .collect();
    let bytes = run_menu(duel, ui, "Attack which monster?", &items);
    duel.set_response(&bytes);
}

fn select_target(duel: &mut Duel, ui: &mut Ui) {
    let items: Vec<Item> = duel
        .candidates()
        .into_iter()
        .enumerate()
        .map(|(i, id)| respond_card(name_of(duel, id), vec![i as u8], id))
        .collect();
    let bytes = run_menu(duel, ui, "Pick a target", &items);
    duel.set_response(&bytes);
}

/// Inspect a graveyard **without leaving the board**: the field stays drawn, and
/// the right column shows the highlighted GY card's details (same panel as
/// everywhere else) above a scrollable list. ↑↓ browse, Esc/Enter return.
fn view_gy(duel: &Duel, ui: &mut Ui, player: usize) {
    let who = if player == current(duel) {
        "Your"
    } else {
        "Opponent's"
    };
    let cards = duel.graveyard(player);
    let you = current(duel);
    let mut sel = 0usize;
    let mut scroll = 0usize;

    loop {
        {
            // The list sits at a FIXED top and scrolls internally, so it never
            // reflows as the selection moves.
            let visible = visible_rows(ui.screen.h, R0_Y);
            scroll = scroll_offset(sel, cards.len(), visible, scroll);

            let scr = &mut ui.screen;
            scr.begin();
            draw_board(scr, duel, Focus::Menu, (0, 0), you, &[], None);

            // List on top (fixed); the selected card's details go BELOW it — its
            // varying height is harmless since nothing is drawn under it.
            let list_bottom = draw_gy_panel(scr, duel, &cards, sel, scroll, R0_Y);
            if let Some(&id) = cards.get(sel) {
                draw_card_panel(scr, id, duel, list_bottom + 1);
            }

            scr.put_bold(
                LABEL_X,
                MENU_Y,
                &format!("▸ {who} Graveyard — {} card(s)", cards.len()),
                Color::White,
            );
            scr.put(
                LABEL_X,
                MENU_Y + 1,
                "↑↓/scroll browse · Esc/Enter/click return · q quit",
                Color::DarkGrey,
            );
            scr.flush(&mut io::stdout());
        }
        match read_input() {
            Input::Key(KeyCode::Up) | Input::Scroll(-1) => sel = sel.saturating_sub(1),
            Input::Key(KeyCode::Down) | Input::Scroll(_) if !cards.is_empty() => {
                sel = (sel + 1).min(cards.len() - 1)
            }
            Input::Key(KeyCode::Enter) | Input::Key(KeyCode::Esc) | Input::Click(..) => return,
            _ => {}
        }
    }
}

/// The graveyard list panel, with the highlighted card marked. `offset` is the
/// first visible row (the caller scrolls it with `scroll_offset`).
fn draw_gy_panel(
    scr: &mut Screen,
    duel: &Duel,
    cards: &[CardId],
    sel: usize,
    offset: usize,
    top_y: u16,
) -> u16 {
    let lines: Vec<PanelLine> = if cards.is_empty() {
        vec![("(empty)".into(), Color::DarkGrey, false)]
    } else {
        cards
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let marker = if i == sel { "›" } else { " " };
                let color = if i == sel {
                    Color::Yellow
                } else {
                    Color::White
                };
                (format!("{marker} {}", hand_label(duel, id)), color, false)
            })
            .collect()
    };
    draw_panel(scr, top_y, " Graveyard ", &lines, offset)
}

fn game_over(duel: &Duel, ui: &mut Ui) {
    render(duel, ui, &result_text(duel), &[]);
    ui.screen.put_bold(
        LABEL_X,
        MENU_Y,
        &format!("▸ {}  (press any key or click to exit)", result_text(duel)),
        Color::Yellow,
    );
    ui.screen.flush(&mut io::stdout());
    // Wait for a real action — ignore hover motion (it would dismiss instantly).
    while let Input::Hover(..) = read_input() {}
}

fn announce_result(duel: &Duel) {
    println!("{}", result_text(duel));
}

fn result_text(duel: &Duel) -> String {
    match duel.result() {
        Some(Winner::Player(p)) => format!("Player {p} wins — {:?}", duel.win_reason()),
        Some(Winner::Draw) => "It's a draw.".into(),
        None => "Game over.".into(),
    }
}

// ===== Terminal setup / input ============================================

/// Enters raw mode + the alternate screen on creation, and restores the terminal
/// on drop — so a normal return (or a panic) always leaves the terminal sane.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Self {
        enable_raw_mode().expect("enable raw mode");
        let mut out = io::stdout();
        let _ = queue!(out, EnterAlternateScreen, EnableMouseCapture, cursor::Hide);
        let _ = out.flush();
        TerminalGuard
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = io::stdout();
        let _ = queue!(out, cursor::Show, DisableMouseCapture, LeaveAlternateScreen);
        let _ = out.flush();
        let _ = disable_raw_mode();
    }
}

/// One user action: a key, a click at a terminal cell, a hover, or a scroll tick.
enum Input {
    Key(KeyCode),
    Click(u16, u16),      // left click at (column, row) — select/act
    RightClick(u16, u16), // right click — open a zone (e.g. a GY pile)
    Hover(u16, u16),      // mouse moved to (column, row) — preview the card there
    Scroll(i8),           // -1 up, +1 down
}

/// Block for the next input event. `q` and Ctrl-C quit the whole demo. Mouse drag
/// and the middle button are ignored.
fn read_input() -> Input {
    loop {
        match event::read() {
            Ok(Event::Key(k)) => {
                if k.kind != KeyEventKind::Press && k.kind != KeyEventKind::Repeat {
                    continue;
                }
                if k.code == KeyCode::Char('q')
                    || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL))
                {
                    quit();
                }
                return Input::Key(k.code);
            }
            Ok(Event::Mouse(m)) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) => return Input::Click(m.column, m.row),
                MouseEventKind::Down(MouseButton::Right) => {
                    return Input::RightClick(m.column, m.row)
                }
                MouseEventKind::Moved => return Input::Hover(m.column, m.row),
                MouseEventKind::ScrollUp => return Input::Scroll(-1),
                MouseEventKind::ScrollDown => return Input::Scroll(1),
                _ => continue,
            },
            _ => continue,
        }
    }
}

/// Restore the terminal and exit immediately (used for `q` / Ctrl-C).
fn quit() -> ! {
    let mut out = io::stdout();
    let _ = queue!(out, cursor::Show, DisableMouseCapture, LeaveAlternateScreen);
    let _ = out.flush();
    let _ = disable_raw_mode();
    std::process::exit(0);
}
