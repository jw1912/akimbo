mod consts;
mod eval;
mod position;
mod movegen;
mod tables;
mod search;

use consts::*;
use position::{Move, Position};
use search::{Engine, go};
use std::{cmp::{max, min}, io::stdin, process, time::Instant};

#[macro_export]
macro_rules! decl {{$($name:ident = $val:expr ),*} => {$(let $name = $val;)*}}

#[macro_export]
macro_rules! decl_mut {{$($name:ident = $val:expr ),*} => {$(let mut $name = $val;)*}}

fn main() {
    println!("{NAME}, created by {AUTHOR}");
    let mut eng = Engine::default();
    eng.pos = Position::from_fen(STARTPOS);
    eng.ttable.resize(1);
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        parse_commands(input.split_whitespace().collect(), &mut eng)
    }
}

fn parse_commands(commands: Vec<&str>, eng: &mut Engine) {
    match *commands.first().unwrap_or(&"oops") {
        "uci" => println!("id name {NAME} {VERSION}\nid author {AUTHOR}\noption name Hash type spin default 128 min 1 max 512\nuciok"),
        "isready" => println!("readyok"),
        "ucinewgame" => {
            eng.pos = Position::from_fen(STARTPOS);
            eng.ttable.clear();
            *eng.htable = Default::default();
        },
        "setoption" => if let ["setoption", "name", "Hash", "value", x] = commands[..] {eng.ttable.resize(x.parse().unwrap())},
        "go" => parse_go(eng, commands),
        "position" => parse_position(&mut eng.pos, commands),
        "perft" => parse_perft(&mut eng.pos, &commands),
        "quit" => process::exit(0),
        _ => {},
    }
}

fn perft(pos: &mut Position, depth: u8) -> u64 {
    let moves = pos.gen::<ALL>();
    let mut positions = 0;
    for &m in &moves.list[0..moves.len] {
        if pos.r#do(m) { continue }
        positions += if depth > 1 { perft(pos, depth - 1) } else { 1 };
        pos.undo();
    }
    positions
}

fn parse_perft(pos: &mut Position, commands: &[&str]) {
    decl!(depth = commands[1].parse().unwrap(), now = Instant::now(), count = perft(pos, depth), time = now.elapsed());
    println!("perft {depth} time {} nodes {count} ({:.2} Mnps)", time.as_millis(), count as f64 / time.as_micros() as f64);
}

fn parse_position(pos: &mut Position, commands: Vec<&str>) {
    decl_mut!(fen = String::new(), move_list = Vec::new(), moves = false);
    for cmd in commands {
        match cmd {
            "position" | "startpos" | "fen" => {}
            "moves" => moves = true,
            _ => if moves { move_list.push(cmd) } else { fen.push_str(format!("{cmd} ").as_str()) }
        }
    }
    *pos = Position::from_fen(if fen.is_empty() { STARTPOS } else { &fen });
    for m in move_list { pos.r#do(Move::from_uci(pos, m)); }
}

fn parse_go(eng: &mut Engine, commands: Vec<&str>) {
    decl_mut!(token = 0, times = [0, 0], mtg = None, alloc = 1000, incs = [0, 0]);
    let tokens = ["go", "movetime", "wtime", "btime", "movestogo", "winc", "binc"];
    for cmd in commands {
        if let Some(x) = tokens.iter().position(|&y| y == cmd) { token = x }
        else if let Ok(val) = cmd.parse::<i64>() {
            match token {
                1 => alloc = val,
                2 | 3 => times[token - 2] = max(val, 0),
                4 => mtg = Some(val),
                5 | 6 => incs[token - 5] = max(val, 0),
                _ => {},
            }
        }
    }
    decl!(side = usize::from(eng.pos.c), mytime = times[side], myinc = incs[side]);
    if mytime != 0 { alloc = min(mytime, mytime / mtg.unwrap_or(25) + 3 * myinc / 4) }
    eng.timing.1 = max(10, alloc - 10) as u128;
    go(eng);
}
