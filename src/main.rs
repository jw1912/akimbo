mod consts;
mod position;
mod movegen;
mod tables;
mod search;

use consts::*;
use position::{Move, Position};
use search::{Engine, go};
use std::{cmp::max, io::stdin, process, time::Instant};

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
        "uci" => println!("id name {NAME} {VERSION}\nid author {AUTHOR}\noption name Hash type spin default 128 min 1 max 512\nuciok"),
        "isready" => println!("readyok"),
        "ucinewgame" => {
            eng.pos = Position::from_fen(STARTPOS);
            eng.hash_table.clear();
            *eng.history_table = Default::default();
        },
        "setoption" => if let ["setoption", "name", "Hash", "value", x] = commands[..] {eng.hash_table.resize(x.parse().unwrap())},
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
        positions += if depth > 1 {perft(pos, depth - 1)} else {1};
        pos.undo();
    }
    positions
}

fn parse_perft(pos: &mut Position, commands: &[&str]) {
    let depth = commands[1].parse().unwrap();
    let now = Instant::now();
    let count = perft(pos, depth);
    let time = now.elapsed();
    println!("perft {depth} time {} nodes {count} ({:.2} Mnps)", time.as_millis(), count as f64 / time.as_micros() as f64);
}

fn parse_position(pos: &mut Position, commands: Vec<&str>) {
    let (mut fen, mut move_list, mut moves) = (String::new(), Vec::new(), false);
    for cmd in commands {
        match cmd {
            "position" | "startpos" | "fen" => {}
            "moves" => moves = true,
            _ => if moves { move_list.push(cmd.to_string()) } else { fen.push_str(format!("{cmd} ").as_str()) }
        }
    }
    *pos = Position::from_fen(if fen.is_empty() {STARTPOS} else {&fen});
    for m in move_list {pos.r#do(Move::from_uci(pos, &m));}
}

fn parse_go(eng: &mut Engine, commands: Vec<&str>) {
    let (mut token, mut times, mut mtg, mut alloc) = (0, [0, 0], None, 1000);
    for command in commands {
        match command {
            "movetime" => token = 1,
            "wtime" => token = 2,
            "btime" => token = 3,
            "movestogo" => token = 4,
            "winc" | "binc" => token = 5,
            _ => match token {
                1 => alloc = command.parse::<i64>().unwrap() as u128 - 10,
                2 => times[0] = max(command.parse::<i64>().unwrap(), 0) as u128,
                3 => times[1] = max(command.parse::<i64>().unwrap(), 0) as u128,
                4 => mtg = Some(command.parse::<u128>().unwrap()),
                _ => {},
            }
        }
    }
    let mytime = times[usize::from(eng.pos.c)];
    if mytime != 0 { alloc = mytime / mtg.unwrap_or(2 * (eng.pos.phase as u128 + 1)) - 10 }
    eng.timing.1 = alloc;
    go(eng);
}
