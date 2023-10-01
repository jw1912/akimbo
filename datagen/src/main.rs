mod thread;
mod util;

use std::{
    env::args,
    sync::atomic::{AtomicBool, Ordering, AtomicU64},
};

use thread::DatagenThread;

pub static STOP: AtomicBool = AtomicBool::new(false);

const NODES_PER_MOVE: u64 = 5_000;

fn main() {
    let threads = args().nth(1).unwrap().parse().unwrap();
    let gpt = args().nth(2).unwrap().parse().unwrap();

    let mut games = Vec::new();
    let mut fens = Vec::new();

    for _ in 0..threads {
        games.push(AtomicU64::new(0));
        fens.push(AtomicU64::new(0));
    }

    std::thread::scope(|s| {
        let games = &games;
        let fens = &fens;

        for num in 0..threads {
            std::thread::sleep(std::time::Duration::from_millis(10));
            s.spawn(move || {
                let mut worker = DatagenThread::new(NODES_PER_MOVE, 8);
                worker.run_datagen(gpt, num, games, fens);
            });
        }

        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let commands = input.split_whitespace().collect::<Vec<_>>();
            if let Some(&"stop") = commands.first() {
                STOP.store(true, Ordering::SeqCst);
                break;
            }
        }
    })
}
