//! akimbo, a UCI compatible chess engine written in Rust.

mod consts;
mod position;
mod movegen;
mod zobrist;
mod tables;
mod eval;
mod search;

use std::{io::stdin, time::Instant};
use consts::*;
use tables::{HashTable, KillerTable};
use position::{Position, State, S};
use movegen::MoveList;
use search::{go, SearchContext};
use zobrist::ZVALS;

macro_rules! parse {($type: ty, $s: expr, $else: expr) => {$s.parse::<$type>().unwrap_or($else)}}

fn main() {
    println!("{NAME}, created by {AUTHOR}");
    let mut pos: Position = parse_fen(STARTPOS);
    let mut ctx: SearchContext = SearchContext::new(HashTable::new(), KillerTable([[0; KILLERS_PER_PLY]; MAX_PLY as usize]));
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(str::trim).collect();
        match *commands.first().unwrap_or(&"oops") {
            "uci" => {
                println!("id name {NAME} {VERSION}");
                println!("id author {AUTHOR}");
                println!("option name UCI_Chess960 type check default false");
                println!("option name Hash type spin default 128 min 1 max 512");
                println!("option name Clear Hash type button");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                pos = parse_fen(STARTPOS);
                ctx.hash_table.clear();
            },
            "setoption" => {
                match commands[..] {
                    ["setoption", "name", "Hash", "value", x] => ctx.hash_table.resize(parse!(usize, x, 1)),
                    ["setoption", "name", "Clear", "Hash"] => ctx.hash_table.clear(),
                    _ => {},
                }
            },
            "go" => parse_go(&mut pos, commands, &mut ctx),
            "position" => parse_position(&mut pos, commands),
            "perft" => parse_perft(&mut pos, &commands),
            _ => println!("unknown command"),
        }
    }
}

fn perft(pos: &mut Position, depth_left: u8) -> u64 {
    let mut moves = MoveList::default();
    pos.gen_moves::<ALL>(&mut moves);
    let mut positions: u64 = 0;
    for m_idx in 0..moves.len {
        let m: u16 = moves.list[m_idx];
        if pos.do_move(m) { continue }
        positions += if depth_left > 1 {perft(pos, depth_left - 1)} else {1};
        pos.undo_move();
    }
    positions
}

fn parse_perft(pos: &mut Position, commands: &[&str]) {
    for d in 0..=parse!(u8, commands[1], 0) {
        let now = Instant::now();
        let count: u64 = perft(pos, d);
        let time = now.elapsed();
        println!("info depth {} time {} nodes {count} Mnps {:.2}", d, time.as_millis(), count as f64 / time.as_micros() as f64);
    }
}

fn parse_go(pos: &mut Position, commands: Vec<&str>, ctx: &mut SearchContext) {
    enum Tokens {None, Depth, Movetime, WTime, BTime, WInc, BInc, MovesToGo}
    let mut token: Tokens = Tokens::None;
    let (mut times, mut moves_to_go, mut depth): ([u64; 2], Option<u16>, i8) = ([0, 0], None, 64);
    ctx.alloc_time = 1000;
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
                    Tokens::Depth => depth = std::cmp::min(parse!(i8, command, 1), 64),
                    Tokens::Movetime => ctx.alloc_time = parse!(i64, command, 1000) as u128 - 10,
                    Tokens::WTime => times[0] = std::cmp::max(parse!(i64, command, 1000), 0) as u64,
                    Tokens::BTime => times[1] = std::cmp::max(parse!(i64, command, 1000), 0) as u64,
                    Tokens::MovesToGo => moves_to_go = Some(parse!(u16, command, 40)),
                    _ => {},
                }
            },
        }
    }
    if times[usize::from(pos.c)] != 0 {
        ctx.alloc_time = times[usize::from(pos.c)] as u128 / (if let Some(mtg) = moves_to_go {mtg as u128} else {2 * (pos.phase as u128 + 1)}) - 10;
    }
    go(pos, depth, ctx);
}

fn parse_position(pos: &mut Position, commands: Vec<&str>) {
    enum Tokens {Nothing, Fen, Moves}
    let mut fen = String::new();
    let mut moves: Vec<String> = Vec::new();
    let mut token: Tokens = Tokens::Nothing;
    for command in commands {
        match command {
            "position" => (),
            "startpos" => *pos = parse_fen(STARTPOS),
            "fen" => token = Tokens::Fen,
            "moves" => token = Tokens::Moves,
            _ => match token {
                Tokens::Nothing => {},
                Tokens::Fen => {fen.push_str(format!("{command} ").as_str());}
                Tokens::Moves => moves.push(command.to_string()),
            },
        }
    }
    if !fen.is_empty() {*pos = parse_fen(&fen)}
    for m in moves {pos.do_move(uci_to_u16(pos, &m));}
}

macro_rules! idx_to_sq {($idx:expr) => {format!("{}{}", char::from_u32(($idx & 7) as u32 + 97).unwrap(), ($idx >> 3) + 1)}}

/// Converts e.g. "a6" to index 5.
fn sq_to_idx(sq: &str) -> u16 {
    let chs: Vec<char> = sq.chars().collect();
    8 * parse!(u16, chs[1].to_string(), 0) + chs[0] as u16 - 105
}

fn u16_to_uci(p: &Position, m: u16) -> String {
    let flag: u16 = m & 0xF000;
    if p.chess960 && (flag == MoveFlags::QS_CASTLE || flag == MoveFlags::KS_CASTLE) {
            let from: u16 = (m >> 6) & 63;
            let rook: u16 = p.castle[(flag == MoveFlags::KS_CASTLE) as usize] as u16 + 56 * (from / 56);
            format!("{}{} ", idx_to_sq!(from), idx_to_sq!(rook))
    } else {
        let promo: &str = if m & 0b1000_0000_0000_0000 > 0 {["n","b","r","q"][((m >> 12) & 0b11) as usize]} else {""};
        format!("{}{}{} ", idx_to_sq!((m >> 6) & 63), idx_to_sq!(m & 63), promo)
    }
}

fn uci_to_u16(pos: &Position, m: &str) -> u16 {
    let l: usize = m.len();
    let from: u16 = sq_to_idx(&m[0..2]);
    let mut to: u16 = sq_to_idx(&m[2..4]);
    let mut castle: u16 = 0;
    if pos.chess960 && pos.sides[usize::from(pos.c)] & (1 << to) > 0 {
        if to == pos.castle[0] as u16 + 56 * (from / 56) {
            to = 2 + 56 * (from / 56);
            castle = MoveFlags::QS_CASTLE;
        } else {
            to = 6 + 56 * (from / 56);
            castle = MoveFlags::KS_CASTLE;
        }
    }
    let mut no_flags: u16 = castle | (from << 6) | to;
    if castle > 0 {return no_flags}
    no_flags |= match m.chars().nth(4).unwrap_or('f') {'n' => 0x8000, 'b' => 0x9000, 'r' => 0xA000, 'q' => 0xB000, _ => 0};
    let mut possible_moves = MoveList::default();
    pos.gen_moves::<ALL>(&mut possible_moves);
    for m_idx in 0..possible_moves.len {
        let um: u16 = possible_moves.list[m_idx];
        if no_flags & TWELVE == um & TWELVE && (l < 5 || no_flags & !TWELVE == um & 0xB000) {return um;}
    }
    panic!("invalid move list!");
}

fn parse_fen(s: &str) -> Position {
    let vec: Vec<&str> = s.split_whitespace().collect();
    let mut pos: Position = Position {
        pieces: [0; 6], sides: [0; 2], squares: [EMPTY as u8; 64], c: false, state: State::default(), scores: S(0, 0),
        nulls: 0, stack: Vec::new(), phase: 0, castle: [0, 7], castle_mask: [15; 64], chess960: false,
    };

    // board
    let mut idx: usize = 63;
    let rows: Vec<&str> = vec[0].split('/').collect();
    for row in rows {
        for ch in row.chars().rev() {
            if ch == '/' { continue }
            if ch.is_numeric() {
                let len: usize = parse!(usize, ch.to_string(), 8);
                idx -= usize::from(idx >= len) * len;
            } else {
                let idx2: usize = ['P','N','B','R','Q','K','p','n','b','r','q','k'].iter().position(|&element| element == ch).unwrap_or(6);
                let (side, pc): (usize, usize) = (usize::from(idx2 > 5), idx2 - 6 * usize::from(idx2 > 5));
                pos.sides[side] ^= 1 << idx;
                pos.pieces[pc] ^= 1 << idx;
                pos.squares[idx] = pc as u8;
                pos.phase += PHASE_VALS[pc];
                pos.scores += SIDE_FACTOR[side] * MATERIAL[pc];
                pos.state.zobrist ^= ZVALS.pieces[side][pc][idx];
                idx -= usize::from(idx > 0);
            }
        }
    }

    // castle rights
    let mut rights: u8 = 0;
    let mut king_col: usize = 4;
    let wkc: u8 = lsb!(pos.pieces[KING] & pos.sides[0]) as u8 & 7;
    let bkc: u8 = lsb!(pos.pieces[KING] & pos.sides[1]) as u8 & 7;
    for ch in vec[2].bytes() {
        rights |= match ch {
            b'Q' => CastleRights::WHITE_QS,
            b'K' => CastleRights::WHITE_KS,
            b'q' => CastleRights::BLACK_QS,
            b'k' => CastleRights::BLACK_KS,
            b'A'..=b'H' => {
                pos.chess960 = true;
                king_col = wkc as usize;
                let rook_col: u8 = ch - b'A';
                if rook_col < wkc {
                    pos.castle[0] = rook_col;
                    CastleRights::WHITE_QS
                } else {
                    pos.castle[1] = rook_col;
                    CastleRights::WHITE_KS
                }
            }
            b'a'..=b'h' => {
                pos.chess960 = true;
                king_col = bkc as usize;
                let rook_col: u8 = ch - b'a';
                if rook_col < bkc {
                    pos.castle[0] = rook_col;
                    CastleRights::BLACK_QS
                } else {
                    pos.castle[1] = rook_col;
                    CastleRights::BLACK_KS
                }
            }
            _ => 0
        };
    }
    pos.state.castle_rights = rights;
    while rights > 0 {
        pos.state.zobrist ^= ZVALS.castle[lsb!(rights) as usize];
        rights &= rights - 1;
    }
    pos.castle_mask[pos.castle[0] as usize] = 7;
    pos.castle_mask[pos.castle[1] as usize] = 11;
    pos.castle_mask[56 + pos.castle[0] as usize] = 13;
    pos.castle_mask[56 + pos.castle[1] as usize] = 14;
    pos.castle_mask[king_col] = 3;
    pos.castle_mask[56 + king_col] = 12;

    // state
    let enp: u16 = if vec[3] == "-" {0} else {sq_to_idx(vec[3])};
    pos.state.en_passant_sq = enp;
    pos.state.halfmove_clock = parse!(u8, vec.get(4).unwrap_or(&"0"), 0);
    pos.c = vec[1] == "b";
    if enp > 0 {pos.state.zobrist ^= ZVALS.en_passant[(enp & 7) as usize]}
    if !pos.c {pos.state.zobrist ^= ZVALS.side;}
    pos
}