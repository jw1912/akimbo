mod util;
mod thread;

use thread::ThreadData;
use std::{
    env::args,
    sync::atomic::{AtomicBool, Ordering},
    thread::spawn,
};

pub static STOP: AtomicBool = AtomicBool::new(false);

const NODES_PER_MOVE: u64 = 5_000;

fn main() {
    let mut handles = Vec::new();

    let threads = args().nth(1).unwrap().parse().unwrap();
    let gpt = args().nth(2).unwrap().parse().unwrap();

    for _ in 0..threads {
        std::thread::sleep(std::time::Duration::from_millis(10));
        handles.push(
            spawn(move || {
                let mut worker = ThreadData::new(NODES_PER_MOVE, 8);
                worker.run_datagen(gpt);
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
