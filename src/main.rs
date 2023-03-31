mod consts;
mod position;
mod movegen;
mod tables;
mod search;

use consts::*;
use position::{Move, Position};
use search::{Engine, go};
use std::{io::stdin, time::Instant};

fn main() {
    println!("{NAME}, created by {AUTHOR}");
    let mut eng = Engine::default();
    eng.pos = Position::from_fen(STARTPOS);
    eng.hash_table.resize(1);
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(str::trim).collect();
        parse_commands(commands, &mut eng)
    }
}

fn parse_commands(commands: Vec<&str>, eng: &mut Engine) {
    match *commands.first().unwrap_or(&"oops") {
        "uci" => println!("id name {NAME} {VERSION}\nid author {AUTHOR}\noption name Hash type spin default 64 min 1 max 512\nuciok"),
        "isready" => println!("readyok"),
        "ucinewgame" => {
            eng.pos = Position::from_fen(STARTPOS);
            eng.hash_table.clear();
        },
        "setoption" => if let ["setoption", "name", "Hash", "value", x] = commands[..] {eng.hash_table.resize(x.parse().unwrap())},
        "go" => parse_go(eng, commands),
        "position" => parse_position(&mut eng.pos, commands),
        "perft" => parse_perft(&mut eng.pos, &commands),
        "quit" => std::process::exit(0),
        _ => {},
    }
}

fn perft(pos: &mut Position, depth_left: u8) -> u64 {
    let moves = pos.gen::<ALL>();
    let mut positions = 0;
    for m_idx in 0..moves.len {
        let m = moves.list[m_idx];
        if pos.r#do(m) { continue }
        positions += if depth_left > 1 {perft(pos, depth_left - 1)} else {1};
        pos.undo();
    }
    positions
}

fn parse_perft(pos: &mut Position, commands: &[&str]) {
    for d in 0..=commands[1].parse().unwrap() {
        let now = Instant::now();
        let count = perft(pos, d);
        let time = now.elapsed();
        println!("info depth {d} time {} nodes {count} Mnps {:.2}", time.as_millis(), count as f64 / time.as_micros() as f64);
    }
}

fn parse_position(pos: &mut Position, commands: Vec<&str>) {
    enum Tokens {Nothing, Fen, Moves}
    let mut fen = String::new();
    let mut moves = Vec::new();
    let mut token = Tokens::Nothing;
    for cmd in commands {
        match cmd {
            "startpos" => *pos = Position::from_fen(STARTPOS),
            "fen" => token = Tokens::Fen,
            "moves" => token = Tokens::Moves,
            _ => match token {
                Tokens::Nothing => {},
                Tokens::Fen => {fen.push_str(format!("{cmd} ").as_str());}
                Tokens::Moves => moves.push(cmd.to_string()),
            },
        }
    }
    if !fen.is_empty() {*pos = Position::from_fen(&fen)}
    for m in moves {pos.r#do(Move::from_uci(pos, &m));}
}

fn parse_go(eng: &mut Engine, commands: Vec<&str>) {
    enum Tokens {None, Movetime, WTime, BTime, WInc, BInc, MovesToGo}
    let mut token = Tokens::None;
    let (mut times, mut mtg, mut skip) = ([0, 0], None, false);
    let mut alloc_time = 1000;
    for command in commands {
        match command {
            "movetime" => token = Tokens::Movetime,
            "wtime" => token = Tokens::WTime,
            "btime" => token = Tokens::BTime,
            "winc" => token = Tokens::WInc,
            "binc" => token = Tokens::BInc,
            "movestogo" => token = Tokens::MovesToGo,
            _ => {
                match token {
                    Tokens::Movetime => {
                        skip = true;
                        alloc_time = command.parse::<i64>().unwrap() as u128 - 10;
                        break;
                    },
                    Tokens::WTime => times[0] = std::cmp::max(command.parse::<i64>().unwrap(), 0) as u128,
                    Tokens::BTime => times[1] = std::cmp::max(command.parse::<i64>().unwrap(), 0) as u128,
                    Tokens::MovesToGo => mtg = Some(command.parse::<u128>().unwrap()),
                    _ => {},
                }
            },
        }
    }
    let mytime = times[usize::from(eng.pos.c)];
    if !skip && mytime != 0 { alloc_time = mytime / mtg.unwrap_or(2 * eng.pos.phase as u128 + 1) - 10 }
    eng.set_time(alloc_time);
    go(eng);
}
