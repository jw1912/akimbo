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
        parse_commands(input.split(' ').map(str::trim).collect(), &mut eng)
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
    for &m in &moves.list[0..moves.len] {
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
    let (mut fen, mut moves, mut token) = (String::new(), Vec::new(), 0);
    for cmd in commands {
        match cmd {
            "startpos" => *pos = Position::from_fen(STARTPOS),
            "fen" => token = 1,
            "moves" => token = 2,
            _ => match token {
                1 => fen.push_str(format!("{cmd} ").as_str()),
                2 => moves.push(cmd.to_string()),
                _ => {}
            },
        }
    }
    if !fen.is_empty() {*pos = Position::from_fen(&fen)}
    for m in moves {pos.r#do(Move::from_uci(pos, &m));}
}

fn parse_go(eng: &mut Engine, commands: Vec<&str>) {
    let (mut token, mut times, mut mtg, mut alloc) = (0, [0, 0], None, 1000);
    for command in commands {
        match command {
            "movetime" => token = 1,
            "wtime" => token = 2,
            "btime" => token = 3,
            "winc" => token = 4,
            "binc" => token = 5,
            "movestogo" => token = 6,
            _ => {
                match token {
                    1 => alloc = command.parse::<i64>().unwrap() as u128 - 10,
                    2 => times[0] = std::cmp::max(command.parse::<i64>().unwrap(), 0) as u128,
                    3 => times[1] = std::cmp::max(command.parse::<i64>().unwrap(), 0) as u128,
                    6 => mtg = Some(command.parse::<u128>().unwrap()),
                    _ => {},
                }
            },
        }
    }
    let mytime = times[usize::from(eng.pos.c)];
    if mytime != 0 { alloc = mytime / mtg.unwrap_or(2 * eng.pos.phase as u128 + 1) - 10 }
    eng.timing.1 = alloc;
    go(eng);
}
