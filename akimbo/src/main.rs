use akimbo::{consts::SIDE, position::Position, search::go, thread::ThreadData};

use std::{io, process, time::Instant};

const FEN_STRING: &str = include_str!("../../resources/fens.txt");
const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    println!("akimbo, created by Jamie Whiting");

    // initialise engine
    let mut pos = Position::from_fen(STARTPOS);
    let mut eng = ThreadData::default();
    eng.tt.resize(16);

    // bench for OpenBench
    if std::env::args().nth(1).as_deref() == Some("bench") {
        let (mut total_nodes, mut total_time) = (0, 0);
        let mut eval = 0i32;
        eng.max_time = 30000;
        let bench_fens = FEN_STRING.split('\n').collect::<Vec<&str>>();
        for fen in bench_fens {
            pos = Position::from_fen(fen);
            eval = eval.wrapping_add(SIDE[pos.stm()] * pos.eval());
            let timer = Instant::now();
            go(&pos, &mut eng, false, 11, 1_000_000.0, u64::MAX);
            total_time += timer.elapsed().as_millis();
            total_nodes += eng.nodes + eng.qnodes;
        }
        println!("Summed Eval: {eval}");
        println!(
            "Bench: {total_nodes} nodes {} nps",
            total_nodes * 1000 / (total_time as u64).max(1)
        );
        return;
    }

    // main uci loop
    loop {
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();
        // got EOF, exit (for OpenBench).
        if bytes_read == 0 {
            break;
        }
        let commands = input.split_whitespace().collect::<Vec<_>>();
        match *commands.first().unwrap_or(&"oops") {
            "uci" => {
                println!(
                    "id name akimbo {}\nid author Jamie Whiting",
                    env!("CARGO_PKG_VERSION")
                );
                println!("option name Threads type spin default 1 min 1 max 1");
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name Clear Hash type button");
                println!("option name UCI_Chess960 type check default false");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                pos = Position::from_fen(STARTPOS);
                eng.tt.clear();
                eng.htable.clear();
            }
            "setoption" => match commands[..] {
                ["setoption", "name", "Hash", "value", x] => eng.tt.resize(x.parse().unwrap()),
                ["setoption", "name", "Clear", "Hash"] => eng.tt.clear(),
                _ => {}
            },
            "go" => {
                let (mut token, mut times, mut mtg, mut alloc, mut incs, mut depth) =
                    (0, [0, 0], 25, 1_000_000, [0, 0], 64);
                let tokens = [
                    "go",
                    "movetime",
                    "wtime",
                    "btime",
                    "movestogo",
                    "winc",
                    "binc",
                    "depth",
                ];
                for cmd in commands {
                    if let Some(x) = tokens.iter().position(|&y| y == cmd) {
                        token = x
                    } else if let Ok(val) = cmd.parse::<i64>() {
                        match token {
                            1 => {
                                alloc = val;
                                mtg = 1;
                            }
                            2 | 3 => times[token - 2] = val.max(0),
                            4 => mtg = val,
                            5 | 6 => incs[token - 5] = val.max(0),
                            7 => depth = val.clamp(0, 64) as i32,
                            _ => {}
                        }
                    }
                }
                let side = pos.stm();
                let (mut time, inc) = (times[side], incs[side]);
                if time != 0 {
                    alloc = time.min(time / mtg + 3 * inc / 4)
                } else {
                    time = alloc
                }
                eng.max_time = (alloc * 2).clamp(1, 1.max(time - 10)) as u128;
                let (bm, _) = go(
                    &pos,
                    &mut eng,
                    true,
                    depth,
                    if mtg == 1 { alloc } else { alloc * 6 / 10 } as f64,
                    u64::MAX,
                );
                println!("bestmove {}", bm.to_uci());
            }
            "position" => {
                let (mut fen, mut move_list, mut moves) = (String::new(), Vec::new(), false);
                for cmd in commands {
                    match cmd {
                        "position" | "startpos" | "fen" => {}
                        "moves" => moves = true,
                        _ => {
                            if moves {
                                move_list.push(cmd)
                            } else {
                                fen.push_str(&format!("{cmd} "))
                            }
                        }
                    }
                }
                pos = Position::from_fen(if fen.is_empty() { STARTPOS } else { &fen });
                eng.stack.clear();
                for m in move_list {
                    eng.stack.push(pos.hash());
                    let possible_moves = pos.movegen::<true>();
                    for mov in possible_moves.iter() {
                        if m == mov.to_uci() {
                            pos.make(*mov);
                        }
                    }
                }
            }
            "perft" => {
                let (depth, now) = (commands[1].parse().unwrap(), Instant::now());
                let count = perft(&pos, depth);
                let time = now.elapsed().as_micros();
                println!(
                    "perft {depth} time {} nodes {count} ({:.2} Mnps)",
                    time / 1000,
                    count as f64 / time as f64
                );
            }
            "quit" => process::exit(0),
            "eval" => println!("eval: {}cp", pos.eval()),
            _ => {}
        }
    }
}

fn perft(pos: &Position, depth: u8) -> u64 {
    let moves = pos.movegen::<true>();
    let mut positions = 0;
    for &m in moves.iter() {
        let mut tmp = *pos;
        if tmp.make(m) {
            continue;
        }
        positions += if depth > 1 { perft(&tmp, depth - 1) } else { 1 };
    }
    positions
}
