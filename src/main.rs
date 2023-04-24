mod consts;
mod eval;
mod position;
mod movegen;
mod tables;
mod search;

use consts::*;
use position::{Move, Position, ZobristVals};
use search::{Engine, go};
use std::{io, process, time::Instant};

fn main() {
    println!("{NAME}, created by {AUTHOR}");
    let mut eng = Engine::default();
    let mut pos = Position::from_fen(STARTPOS, &eng.zvals);
    eng.ttable.resize(1);
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        parse_commands(input.split_whitespace().collect(), &mut pos, &mut eng);
    }
}

fn parse_commands(commands: Vec<&str>, pos: &mut Position, eng: &mut Engine) {
    match *commands.first().unwrap_or(&"oops") {
        "uci" => println!("id name {NAME} {VERSION}\nid author {AUTHOR}\noption name Hash type spin default 128 min 1 max 512\nuciok"),
        "isready" => println!("readyok"),
        "ucinewgame" => {
            *pos = Position::from_fen(STARTPOS, &eng.zvals);
            eng.ttable.clear();
            *eng.htable = Default::default();
        },
        "setoption" => if let ["setoption", "name", "Hash", "value", x] = commands[..] {eng.ttable.resize(x.parse().unwrap())},
        "go" => parse_go(pos, eng, commands),
        "position" => parse_position(pos, commands, &eng.zvals),
        "perft" => parse_perft(pos, &commands, &eng.zvals),
        "quit" => process::exit(0),
        _ => {},
    }
}

fn perft(pos: &Position, depth: u8, zvals: &ZobristVals) -> u64 {
    let moves = pos.gen::<ALL>();
    let mut positions = 0;
    for &m in &moves.list[0..moves.len] {
        let mut tmp = *pos;
        if tmp.make(m, zvals) { continue }
        positions += if depth > 1 { perft(&tmp, depth - 1, zvals) } else { 1 };
    }
    positions
}

fn parse_perft(pos: &mut Position, commands: &[&str], zvals: &ZobristVals) {
    let (depth, now) = (commands[1].parse().unwrap(), Instant::now());
    let count = perft(pos, depth, zvals);
    let time = now.elapsed();
    println!("perft {depth} time {} nodes {count} ({:.2} Mnps)", time.as_millis(), count as f64 / time.as_micros() as f64);
}

fn parse_position(pos: &mut Position, commands: Vec<&str>, zvals: &ZobristVals) {
    let (mut fen, mut move_list, mut moves) = (String::new(), Vec::new(), false);
    for cmd in commands {
        match cmd {
            "position" | "startpos" | "fen" => {}
            "moves" => moves = true,
            _ => if moves { move_list.push(cmd) } else { fen.push_str(format!("{cmd} ").as_str()) }
        }
    }
    *pos = Position::from_fen(if fen.is_empty() { STARTPOS } else { &fen }, zvals);
    for m in move_list { pos.make(Move::from_uci(pos, m), zvals); }
}

fn parse_go(pos: &Position, eng: &mut Engine, commands: Vec<&str>) {
    let (mut token, mut times, mut mtg, mut alloc, mut incs) = (0, [0, 0], None, 1000, [0, 0]);
    let tokens = ["go", "movetime", "wtime", "btime", "movestogo", "winc", "binc"];
    for cmd in commands {
        if let Some(x) = tokens.iter().position(|&y| y == cmd) { token = x }
        else if let Ok(val) = cmd.parse::<i64>() {
            match token {
                1 => alloc = val,
                2 | 3 => times[token - 2] = val.max(0),
                4 => mtg = Some(val),
                5 | 6 => incs[token - 5] = val.max(0),
                _ => {},
            }
        }
    }
    let side = usize::from(pos.c);
    let (mytime, myinc) = (times[side], incs[side]);
    if mytime != 0 { alloc = mytime.min(mytime / mtg.unwrap_or(25) + 3 * myinc / 4) }
    eng.timing.1 = 10.max(alloc - 10) as u128;
    go(pos, eng);
}
