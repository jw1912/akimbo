mod util;
mod thread;

use thread::ThreadData;
use std::{sync::atomic::{AtomicBool, Ordering}, thread::{spawn, sleep}};

pub static STOP: AtomicBool = AtomicBool::new(false);

fn main() {
    let mut handles = Vec::new();
    for _ in 0..4 {
        sleep(std::time::Duration::from_millis(500));
        handles.push(
            spawn(move || {
                let mut worker = ThreadData::new(20_000, 8);
                worker.run_datagen(10000);
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
