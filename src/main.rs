//! akimbo, a UCI compatible chess engine written in Rust.

#![deny(missing_docs)]

/// Contains all constant and static **immutable** values used in the engine.
mod consts;
/// Contains all methods that mutate the global POS (apart from parsing positions).
pub mod position;
/// Conatins pseudo-legal move generation code.
pub mod movegen;
/// Contains hash and killer move tables.
pub mod hash;
/// Contains the main engine code for searching positions.
pub mod search;

use std::io::stdin;
use std::time::Instant;
use consts::*;
use hash::{tt_clear, tt_resize, zobrist, kt_clear};
use position::{POS, MoveList, do_move, undo_move, GameState, calc};
use movegen::{gen_moves, ALL};
use search::{DEPTH, TIME, go};

macro_rules! parse {($type: ty, $s: expr, $else: expr) => {$s.parse::<$type>().unwrap_or($else)}}

/// Main loop waits until receiving the "uci" command.
fn main() {
    println!("akimbo, created by Jamie Whiting");
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(|v| v.trim()).collect();
        if commands[0] == "uci" {uci_run()}
    }
}

/// Runs a fixed time (1 second) search on a small collection of FENs,
/// used to check for any glaring bugs introduced by new search techniques.
fn performance(commands: Vec<&str>) {
    tt_resize(128);
    let time: u128 = if commands.len() >= 2 {parse!(u128, commands[1], 1000)} else {1000};
    let now = Instant::now();
    for fen in _POSITIONS {
        unsafe {TIME = time;}
        kt_clear();
        parse_fen(fen);
        println!("\n===Search Report===\nfen: {}", fen);
        go();
    }
    println!("Total time: {}ms", now.elapsed().as_millis());
    ucinewgame();
}

/// Runs a perft on the current position to a given depth.
fn perft<const ROOT: bool>(depth_left: u8) -> u64 {
    if depth_left == 0 { return 1 }
    let mut moves = MoveList::default();
    gen_moves::<ALL>(&mut moves);
    let mut positions: u64 = 0;
    for m_idx in 0..moves.len {
        let m: u16 = moves.list[m_idx];
        if do_move(m) { continue }
        let count: u64 = perft::<false>(depth_left - 1);
        if ROOT {println!("{}: {}", u16_to_uci(&m), count)}
        positions += count;
        undo_move();
    }
    positions
}

/// Main uci loop, taking in input from the command line.
fn uci_run() {
    // init position and hash table
    parse_fen(STARTPOS);
    tt_resize(1);
    // uci preamble
    println!("id name akimbo {}", VERSION);
    println!("id author {}", AUTHOR);
    println!("option name Hash type spin default 128 min 1 max 512");
    println!("option name Clear Hash type button");
    println!("uciok");
    // await commands
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(|v| v.trim()).collect();
        parse_commands(commands);
    }
}

/// Parses the uci commands received.
fn parse_commands(commands: Vec<&str>) {
    match commands[0] {
        "isready" => println!("readyok"),
        "ucinewgame" => ucinewgame(),
        "go" => parse_go(commands),
        "position" => parse_position(commands),
        "setoption" => parse_setoption(commands),
        "perft" => parse_perft(commands),
        "performance" => performance(commands),
        _ => {},
    };
}

/// Resets position to starting position, clears hash and killer move tables.
fn ucinewgame() {
    parse_fen(STARTPOS);
    tt_clear();
    kt_clear();
}

/// Runs a perft search to a specified depth.
fn parse_perft(commands: Vec<&str>) {
    let now = Instant::now();
    let count: u64 = perft::<true>(parse!(u8, commands[1], 0));
    println!("leaf count: {count} ({:.2} ML/sec)", count as f64 / now.elapsed().as_micros() as f64);
}

/// Parses "go ..." and runs the requested search based on this.
fn parse_go( commands: Vec<&str>) {
    #[derive(PartialEq)]
    enum Tokens {None, Depth, Movetime, WTime, BTime, WInc, BInc, MovesToGo}
    let mut token: Tokens = Tokens::None;
    let (mut times, mut moves_to_go): ([u64; 2], Option<u16>) = ([0, 0], None);
    for command in commands {
        match command {
            "depth" => token = Tokens::Depth,
            "movetime" => token = Tokens::Movetime,
            "wtime" => token = Tokens::WTime,
            "btime" => token = Tokens::BTime,
            "winc" => token = Tokens::WInc,
            "binc" => token = Tokens::BInc,
            "movestogo" => token = Tokens::MovesToGo,
            _ => {
                match token {
                    Tokens::None => {},
                    Tokens::Depth => unsafe {
                        DEPTH = parse!(i8, command, 1);
                        TIME = u128::MAX;
                    },
                    Tokens::Movetime => unsafe{TIME = parse!(i64, command, 1000) as u128 - 10}
                    Tokens::WTime => times[0] = std::cmp::max(parse!(i64, command, 1000), 0) as u64,
                    Tokens::BTime => times[1] = std::cmp::max(parse!(i64, command, 1000), 0) as u64,
                    Tokens::MovesToGo => moves_to_go = Some(parse!(u16, command, 40)),
                    _ => {},
                }
            },
        }
    }
    unsafe {
    if times[POS.side_to_move] != 0 {
        TIME = if let Some(mtg) = moves_to_go {
            times[POS.side_to_move] as u128 / mtg as u128
        } else {
            times[POS.side_to_move] as u128 / (2 * (POS.state.phase as u128 + 1))
        } - 10;
    }}
    go();
}

/// Parses "position ...".
fn parse_position(commands: Vec<&str>) {
    enum Tokens {Nothing, Fen, Moves}
    let mut fen = String::from("");
    let mut moves: Vec<String> = Vec::new();
    let mut token: Tokens = Tokens::Nothing;
    for command in commands {
        match command {
            "position" => (),
            "startpos" => parse_fen(STARTPOS),
            "kiwipete" => parse_fen(KIWIPETE),
            "lasker" => parse_fen(LASKER),
            "fen" => token = Tokens::Fen,
            "moves" => token = Tokens::Moves,
            _ => match token {
                Tokens::Nothing => {},
                Tokens::Fen => {fen.push_str(format!("{command} ").as_str());}
                Tokens::Moves => moves.push(command.to_string()),
            },
        }
    }
    if !fen.is_empty() {parse_fen(&fen)}
    for m in moves {do_move(uci_to_u16(&m));}
}

/// Parses "setoption name ...".
fn parse_setoption(commands: Vec<&str>) {
    match commands[..] {
        ["setoption", "name", "Hash", "value", x] => tt_resize(parse!(usize, x, 1)),
        ["setoption", "name", "Clear", "Hash"] => tt_clear(),
        _ => {},
    }
}

macro_rules! idx_to_sq {($idx:expr) => {format!("{}{}", FILES[($idx & 7) as usize], ($idx >> 3) + 1)}}

/// Converts e.g. "a6" to index 5.
fn sq_to_idx(sq: &str) -> u16 {
    let chs: Vec<char> = sq.chars().collect();
    8 * parse!(u16, chs[1].to_string(), 0) + chs[0] as u16 - 105
}

/// Converts a u16 representation of a move to UCI format.
pub fn u16_to_uci(m: &u16) -> String {
    let promo: &str = if m & PROMO_BIT > 0 {PROMOS[((m >> 12) & 0b11) as usize]} else {""};
    format!("{}{}{} ", idx_to_sq!((m >> 6) & 0b111111), idx_to_sq!(m & 0b111111), promo)
}

/// Converts standard UCI move notation to the usual u16 format used by the engine.
pub fn uci_to_u16(m: &str) -> u16 {
    let l: usize = m.len();
    let from: u16 = sq_to_idx(&m[0..2]);
    let to: u16 = sq_to_idx(&m[2..4]);
    let mut no_flags: u16 = (from << 6) | to;
    no_flags |= match m.chars().nth(4).unwrap_or('f') {'n' => 0x8000, 'b' => 0x9000, 'r' => 0xA000, 'q' => 0xB000, _ => 0,};
    let mut possible_moves = MoveList::default();
    gen_moves::<ALL>(&mut possible_moves);
    for m_idx in 0..possible_moves.len {
        let um: u16 = possible_moves.list[m_idx];
        if no_flags & TWELVE == um & TWELVE && (l < 5 || no_flags & !TWELVE == um & 0xB000) {return um;}
    }
    panic!("invalid move list!");
}

/// Parses a FEN string and sets the global POS to it.
pub fn parse_fen(s: &str) {
    unsafe {
    let vec: Vec<&str> = s.split_whitespace().collect();

    // reset POS
    POS.pieces = [0; 6];
    POS.squares = [EMPTY as u8; 64];
    POS.sides = [0; 2];
    POS.side_to_move = match vec[1] { "w" => WHITE, "b" => BLACK, _ => panic!("") };

    // main part of fen -> bitboards
    let mut idx: usize = 63;
    let rows: Vec<&str> = vec[0].split('/').collect();
    for row in rows {
        for ch in row.chars().rev() {
            if ch == '/' { continue }
            if !ch.is_numeric() {
                let idx2: usize = PIECES.iter().position(|&element| element == ch).unwrap_or(6);
                let (col, pc): (usize, usize) = ((idx2 > 5) as usize, idx2 - 6 * ((idx2 > 5) as usize));
                toggle!(col, pc, 1 << idx);
                POS.squares[idx] = pc as u8;
                idx -= (idx > 0) as usize;
            } else {
                let len: usize = parse!(usize, ch.to_string(), 8);
                idx -= (idx >= len) as usize * len;
            }
        }
    }

    // calculate state
    let mut castle_rights: u8 = CastleRights::NONE;
    for ch in vec[2].chars() {
        castle_rights |= match ch {'Q' => CastleRights::WHITE_QS, 'K' => CastleRights::WHITE_KS, 'q' => CastleRights::BLACK_QS, 'k' => CastleRights::BLACK_KS, _ => 0,};
    }
    let en_passant_sq: u16 = if vec[3] == "-" {0} else {sq_to_idx(vec[3])};
    let halfmove_clock: u8 = parse!(u8, vec[4], 0);
    let (phase, mg, eg): (i16, i16, i16) = calc();

    // set state
    POS.state = GameState {zobrist: 0, phase, mg, eg,en_passant_sq, halfmove_clock, castle_rights};
    POS.fullmove_counter = parse!(u16, vec[5], 1);
    POS.state.zobrist = zobrist::calc();
    POS.stack.clear();
    }
}