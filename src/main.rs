pub mod consts;
pub mod position;
pub mod movegen;
pub mod hash;
pub mod eval;
pub mod search;

use std::io;
use consts::*;
use hash::*;
use position::*;
use movegen::*;
use search::*;
use std::time::Instant;

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    println!("akimbo, created by Jamie Whiting");
    parse_fen(STARTPOS);
    tt_resize(128 * 1024 * 1024);
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(|v| v.trim()).collect();
        if commands[0] == "uci" {uci_run()}
    }
}

fn perft(depth_left: u8) -> u64 {
    if depth_left == 0 { return 1 }
    let mut moves = MoveList::default();
    gen_moves::<All>(&mut moves);
    let mut positions: u64 = 0;
    for m_idx in 0..moves.len {
        let m = moves.list[m_idx];
        let invalid = do_move(m);
        if invalid { continue }
        let score = perft(depth_left - 1);
        positions += score;
        undo_move();
    }
    positions
}

fn uci_run() {
    println!("id name aimbo {}", VERSION);
    println!("id author {}", AUTHOR);
    println!("uciok");
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(|v| v.trim()).collect();
        run_commands(commands);
    }
}

fn run_commands(commands: Vec<&str>) {
    match commands[0] {
        "isready" => println!("readyok"),
        "ucinewgame" => {
            parse_fen(STARTPOS);
            tt_clear();
        },
        "go" => parse_go(commands),
        "position" => parse_position(commands),
        _ => {},
    };
}

fn parse_go( commands: Vec<&str>) {
    #[derive(PartialEq)]
    enum Tokens {None, Depth, Perft}
    let mut token = Tokens::None;
    let mut perft_depth = 0;
    for command in commands {
        match command {
            "depth" => token = Tokens::Depth,
            "perft" => token = Tokens::Perft,
            _ => {
                match token {
                    Tokens::None => {},
                    Tokens::Depth => unsafe{DEPTH = command.parse::<i8>().unwrap_or(1)},
                    Tokens::Perft => perft_depth = command.parse::<u8>().unwrap_or(1),
                }
            },
        }
    }
    match token {
        Tokens::Perft => {
            let now = Instant::now();
            let mut total = 0;
            for d in 0..perft_depth {
                let count = perft(d + 1);
                total += count;
                println!("info depth {} nodes {}", d + 1, count)
            }
            let elapsed = now.elapsed().as_micros();
            println!("Leaf count: {total} ({:.2} ML/sec)", total as f64 / elapsed as f64);
        }
        Tokens::Depth => {
            let best_move = go();
            println!("bestmove {}", u16_to_uci(&best_move));
        } 
        Tokens::None => {}
    }
}

fn parse_position(commands: Vec<&str>) {
    enum Tokens {Nothing, Fen, Moves}
    let mut fen = String::from("");
    let mut moves: Vec<String> = Vec::new();
    let mut token = Tokens::Nothing;
    for command in commands {
        match command {
            "position" => (),
            "startpos" => parse_fen(STARTPOS),
            "fen" => token = Tokens::Fen,
            "moves" => token = Tokens::Moves,
            _ => match token {
                Tokens::Nothing => {},
                Tokens::Fen => {
                    fen.push_str(command);
                    fen.push(' ');
                }
                Tokens::Moves => moves.push(command.to_string()),
            },
        }
    }
    if !fen.is_empty() {parse_fen(&fen)}
    for m in moves {do_move(uci_to_u16(&m));}
}