use crate::frc::Castling;
use crate::position::Position;
use crate::search::go;
use crate::tables::{HashTable, HistoryTable};
use crate::thread::ThreadData;
use crate::util::STARTPOS;

use std::{io, process, sync::atomic::AtomicBool, time::Instant};

const FEN_STRING: &str = include_str!("../resources/fens.txt");

pub fn run_uci() {
    // initialise engine
    let mut castling = Castling::default();
    let mut pos = Position::from_fen(STARTPOS, &mut castling);
    let mut stack = Vec::new();
    let mut tt = HashTable::default();
    let mut htable = HistoryTable::default();
    let mut threads = 1;
    tt.resize(16, 1);

    // bench for OpenBench
    if let Some("bench") = std::env::args().nth(1).as_deref() {
        run_bench(&tt, stack, &htable);
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
            "uci" => preamble(),
            "isready" => println!("readyok"),
            "ucinewgame" => {
                pos = Position::from_fen(STARTPOS, &mut castling);
                tt.clear(threads);
                htable.clear();
            }
            "setoption" => match commands[..] {
                ["setoption", "name", "Hash", "value", x] => tt.resize(x.parse().unwrap(), threads),
                ["setoption", "name", "Clear", "Hash"] => tt.clear(threads),
                ["setoption", "name", "Threads", "value", x] => threads = x.parse().unwrap(),
                _ => {}
            },
            "go" => handle_go(
                commands,
                &pos,
                &castling,
                stack.clone(),
                &mut htable,
                &mut stored_message,
                &tt,
                threads,
            ),
            "position" => set_position(commands, &mut pos, &mut stack, &mut castling),
            "perft" => run_perft(commands, &pos, &castling),
            "quit" => process::exit(0),
            "eval" => {
                let mut accs = Default::default();
                pos.refresh(&mut accs);
                println!("eval: {}cp", pos.eval(&accs));
            },
            _ => {}
        }
    }
}

fn handle_search_input(abort: &AtomicBool) -> Option<String> {
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
                abort.store(true, std::sync::atomic::Ordering::Relaxed);
                return None;
            }
            _ => return Some(input),
        };
    }
}

fn perft<const ROOT: bool>(pos: &Position, castling: &Castling, depth: u8) -> u64 {
    let moves = pos.movegen::<true>(castling);
    let mut positions = 0;
    for &m in moves.iter() {
        let mut tmp = *pos;
        if tmp.make(m, castling) {
            continue;
        }

        let count = if depth > 1 {
            perft::<false>(&tmp, castling, depth - 1)
        } else {
            1
        };

        if ROOT {
            println!("{}: {count}", m.to_uci(castling));
        }

        positions += count;
    }
    positions
}

fn run_perft(commands: Vec<&str>, pos: &Position, castling: &Castling) {
    let depth = commands[1].parse().unwrap();
    let now = Instant::now();
    let count = perft::<true>(pos, castling, depth);
    let time = now.elapsed().as_micros();
    println!(
        "perft {depth} time {} nodes {count} ({:.2} Mnps)",
        time / 1000,
        count as f64 / time as f64
    );
}

fn run_bench(tt: &HashTable, stack: Vec<u64>, htable: &HistoryTable) {
    let abort = AtomicBool::new(false);
    let mut eng = ThreadData::new(&abort, tt, stack, htable.clone(), Castling::default());
    let mut total_nodes = 0;
    let mut total_time = 0;
    let mut eval = 0i32;
    eng.max_time = 30000;
    let bench_fens = FEN_STRING.split('\n').collect::<Vec<&str>>();
    for fen in bench_fens {
        let pos = Position::from_fen(fen, &mut eng.castling);
        let mut accs = Default::default();
        pos.refresh(&mut accs);
        eval = eval.wrapping_add([1, -1][pos.stm()] * pos.eval(&accs));
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
}

fn preamble() {
    println!("id name akimbo {}", env!("CARGO_PKG_VERSION"));
    println!("id author Jamie Whiting");
    println!("option name Threads type spin default 1 min 1 max 512");
    println!("option name Hash type spin default 16 min 1 max 1048576");
    println!("option name Clear Hash type button");
    println!("option name UCI_Chess960 type check default false");
    println!("uciok");
}

fn set_position(commands: Vec<&str>, pos: &mut Position, stack: &mut Vec<u64>, castling: &mut Castling) {
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

    *pos = Position::from_fen(if fen.is_empty() { STARTPOS } else { &fen }, castling);
    stack.clear();

    for m in move_list {
        stack.push(pos.hash());
        let possible_moves = pos.movegen::<true>(castling);

        for mov in possible_moves.iter() {
            if m == mov.to_uci(castling) {
                pos.make(*mov, castling);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_go(
    commands: Vec<&str>,
    pos: &Position,
    castling: &Castling,
    stack: Vec<u64>,
    htable: &mut HistoryTable,
    stored_message: &mut Option<String>,
    tt: &HashTable,
    threads: usize,
) {
    let mut token = 0;
    let mut times = [0, 0];
    let mut mtg = 25;
    let mut alloc = 1_000_000;
    let mut incs = [0, 0];
    let mut depth = 64;
    let mut nodes = u64::MAX;

    let tokens = [
        "go",
        "movetime",
        "wtime",
        "btime",
        "movestogo",
        "winc",
        "binc",
        "depth",
        "nodes",
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
                8 => nodes = val as u64,
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

    let abort = AtomicBool::new(false);

    let hard_bound = (alloc * 2).clamp(1, 1.max(time - 10)) as u128;
    let soft_bound = if mtg == 1 { alloc } else { alloc * 6 / 10 };

    // main search thread
    let mut eng = ThreadData::new(&abort, tt, stack.clone(), htable.clone(), *castling);
    eng.max_time = hard_bound;
    eng.max_nodes = nodes;

    std::thread::scope(|s| {
        s.spawn(|| {
            let (bm, _) = go(pos, &mut eng, true, depth, soft_bound as f64, u64::MAX);

            println!("bestmove {}", bm.to_uci(castling));
        });

        for _ in 0..(threads - 1) {
            let mut sub = ThreadData::new(&abort, tt, stack.clone(), htable.clone(), *castling);
            sub.max_time = hard_bound;
            s.spawn(move || go(pos, &mut sub, false, depth, soft_bound as f64, u64::MAX));
        }

        *stored_message = handle_search_input(&abort);
    });

    *htable = eng.htable;
    tt.age_up();
}