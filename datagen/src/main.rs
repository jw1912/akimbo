mod util;
mod thread;

use thread::ThreadData;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::spawn,
};

pub static STOP: AtomicBool = AtomicBool::new(false);

const GAMES_PER_THREAD: usize = 40_000;
const NODES_PER_MOVE: u64 = 5_000;
const THREADS: usize = 4;

fn main() {
    let mut handles = Vec::new();

    for _ in 0..THREADS {
        handles.push(
            spawn(move || {
                let mut worker = ThreadData::new(NODES_PER_MOVE, 8);
                worker.run_datagen(GAMES_PER_THREAD as u64);
            })
        );
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

    for handle in handles {
        handle.join().unwrap();
    }
}
