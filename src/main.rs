mod util;
mod position;
mod search;

use crate::{position::{Move, MoveList, Position}, search::{Engine, go}};
use std::{io, process, time::Instant};

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    println!("akimbo, created by Jamie Whiting");
    let mut pos = Position::from_fen(STARTPOS);
    let mut eng = Engine {
        timing: Instant::now(), max_time: 0, abort: false,
        tt: Vec::new(), tt_age: 0,
        htable: Box::new([[[0; 64]; 6]; 2]), hmax: 1,
        ktable: Box::new([[Move::default(); 2]; 96]),
        stack: Vec::with_capacity(96),
        nodes: 0, ply: 0, best_move: Move::default(),
        pv_table: Box::new([MoveList::default(); 96]),
    };
    eng.resize_tt(16);
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let commands = input.split_whitespace().collect::<Vec<_>>();
        match *commands.first().unwrap_or(&"oops") {
            "uci" => {
                println!("id name akimbo {}\nid author Jamie Whiting", env!("CARGO_PKG_VERSION"));
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name Clear Hash type button");
                println!("option name UCI_Chess960 type check default false");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "ucinewgame" => {
                pos = Position::from_fen(STARTPOS);
                eng.clear_tt();
                eng.htable = Box::new([[[0; 64]; 6]; 2]);
                eng.hmax = 1;
            },
            "setoption" => match commands[..] {
                ["setoption", "name", "Hash", "value", x] => eng.resize_tt(x.parse().unwrap()),
                ["setoption", "name", "Clear", "Hash"] => eng.clear_tt(),
                _ => {}
            },
            "go" => {
                let (mut token, mut times, mut mtg, mut alloc, mut incs) = (0, [0, 0], 25, 1000, [0, 0]);
                let tokens = ["go", "movetime", "wtime", "btime", "movestogo", "winc", "binc"];
                for cmd in commands {
                    if let Some(x) = tokens.iter().position(|&y| y == cmd) { token = x }
                    else if let Ok(val) = cmd.parse::<i64>() {
                        match token {
                            1 => alloc = val,
                            2 | 3 => times[token - 2] = val.max(0),
                            4 => mtg = val,
                            5 | 6 => incs[token - 5] = val.max(0),
                            _ => {},
                        }
                    }
                }
                let side = usize::from(pos.c);
                let (time, inc) = (times[side], incs[side]);
                if time != 0 { alloc = time.min(time / mtg + 3 * inc / 4) }
                eng.max_time = 10.max(alloc - 10) as u128;
                go(&pos, &mut eng);
            },
            "position" => {
                let (mut fen, mut move_list, mut moves) = (String::new(), Vec::new(), false);
                for cmd in commands {
                    match cmd {
                        "position" | "startpos" | "fen" => {}
                        "moves" => moves = true,
                        _ => if moves { move_list.push(cmd) } else { fen.push_str(&format!("{cmd} ")) }
                    }
                }
                pos = Position::from_fen(if fen.is_empty() { STARTPOS } else { &fen });
                eng.stack.clear();
                for m in move_list {
                    eng.stack.push(pos.hash());
                    pos.make(Move::from_uci(&pos, m));
                }
            },
            "perft" => {
                let (depth, now) = (commands[1].parse().unwrap(), Instant::now());
                let count = perft(&pos, depth);
                let time = now.elapsed().as_micros();
                println!("perft {depth} time {} nodes {count} ({:.2} Mnps)", time / 1000, count as f64 / time as f64);
            },
            "quit" => process::exit(0),
            _ => {},
        }
    }
}

fn perft(pos: &Position, depth: u8) -> u64 {
    let moves = pos.movegen::<true>();
    let mut positions = 0;
    for &m in &moves.list[0..moves.len] {
        let mut tmp = *pos;
        if tmp.make(m) { continue }
        positions += if depth > 1 { perft(&tmp, depth - 1) } else { 1 };
    }
    positions
}
