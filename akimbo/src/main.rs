use akimbo::{
    consts::SIDE,
    position::Position,
    search::go,
    tables::{HashTable, HistoryTable},
    thread::{Stop, ThreadData},
};

use std::{io, process, time::Instant};

const FEN_STRING: &str = include_str!("../../resources/fens.txt");
const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    println!("akimbo, created by Jamie Whiting");

    // initialise engine
    let mut pos = Position::from_fen(STARTPOS);
    let mut stack = Vec::new();
    let mut tt = HashTable::default();
    let mut htable = HistoryTable::default();
    let mut threads = 1;
    tt.resize(16);

    // bench for OpenBench
    if std::env::args().nth(1).as_deref() == Some("bench") {
        let mut eng = ThreadData::new(&tt, stack, htable);
        let mut total_nodes = 0;
        let mut total_time = 0;
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
            tt.age_up();
        }
        println!("Summed Eval: {eval}");
        println!(
            "Bench: {total_nodes} nodes {} nps",
            total_nodes * 1000 / (total_time as u64).max(1)
        );
        return;
    }

    let mut stored_message: Option<String> = None;

    // main uci loop
    loop {
        let input = if let Some(msg) = stored_message {
            msg.clone()
        } else {
            let mut input = String::new();
            let bytes_read = io::stdin().read_line(&mut input).unwrap();

            // got EOF, exit (for OpenBench).
            if bytes_read == 0 {
                break;
            }

            input
        };

        stored_message = None;

        let commands = input.split_whitespace().collect::<Vec<_>>();

        match *commands.first().unwrap_or(&"oops") {
            "uci" => {
                println!(
                    "id name akimbo {}\nid author Jamie Whiting",
                    env!("CARGO_PKG_VERSION")
                );
                println!("option name Threads type spin default 1 min 1 max 512");
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name Clear Hash type button");
                println!("option name UCI_Chess960 type check default false");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                pos = Position::from_fen(STARTPOS);
                tt.clear();
                htable.clear();
            }
            "setoption" => match commands[..] {
                ["setoption", "name", "Hash", "value", x] => tt.resize(x.parse().unwrap()),
                ["setoption", "name", "Clear", "Hash"] => tt.clear(),
                ["setoption", "name", "Threads", "value", x] => threads = x.parse().unwrap(),
                _ => {}
            },
            "go" => {
                let mut token = 0;
                let mut times = [0, 0];
                let mut mtg = 25;
                let mut alloc = 1_000_000;
                let mut incs = [0, 0];
                let mut depth = 64;

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
                let mut time = times[side];
                let inc = incs[side];
                if time != 0 {
                    alloc = time.min(time / mtg + 3 * inc / 4)
                } else {
                    time = alloc
                }

                Stop::store(false);

                let hard_bound = (alloc * 2).clamp(1, 1.max(time - 10)) as u128;
                let soft_bound = if mtg == 1 { alloc } else { alloc * 6 / 10 };

                // main search thread
                let mut eng = ThreadData::new(&tt, stack.clone(), htable.clone());
                eng.max_time = hard_bound;

                std::thread::scope(|s| {
                    s.spawn(|| {
                        let (bm, _) = go(&pos, &mut eng, true, depth, soft_bound as f64, u64::MAX);

                        println!("bestmove {}", bm.to_uci());
                    });

                    for _ in 0..(threads - 1) {
                        let mut sub = ThreadData::new(&tt, stack.clone(), htable.clone());
                        sub.max_time = hard_bound;
                        s.spawn(move || {
                            go(&pos, &mut sub, false, depth, soft_bound as f64, u64::MAX)
                        });
                    }

                    stored_message = handle_search_input();
                });

                htable = eng.htable.clone();
                tt.age_up();
            }
            "position" => {
                let mut fen = String::new();
                let mut move_list = Vec::new();
                let mut moves = false;

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
                stack.clear();

                for m in move_list {
                    stack.push(pos.hash());
                    let possible_moves = pos.movegen::<true>();

                    for mov in possible_moves.iter() {
                        if m == mov.to_uci() {
                            pos.make(*mov);
                        }
                    }
                }
            }
            "perft" => {
                let depth = commands[1].parse().unwrap();
                let now = Instant::now();
                let count = perft::<true>(&pos, depth);
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

fn handle_search_input() -> Option<String> {
    loop {
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();

        // got EOF, exit (for OpenBench).
        if bytes_read == 0 {
            process::exit(0);
        }

        match input.as_str().trim() {
            "isready" => println!("readyok"),
            "quit" => process::exit(0),
            "stop" => {
                Stop::store(true);
                return None;
            }
            _ => return Some(input),
        };
    }
}

fn perft<const ROOT: bool>(pos: &Position, depth: u8) -> u64 {
    let moves = pos.movegen::<true>();
    let mut positions = 0;
    for &m in moves.iter() {
        let mut tmp = *pos;
        if tmp.make(m) {
            continue;
        }

        let count = if depth > 1 {
            perft::<false>(&tmp, depth - 1)
        } else {
            1
        };

        if ROOT {
            println!("{}: {count}", m.to_uci());
        }

        positions += count;
    }
    positions
}
