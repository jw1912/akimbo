mod util;
mod thread;

use thread::ThreadData;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    sync::atomic::{AtomicBool, Ordering},
    thread::spawn,
};

pub static STOP: AtomicBool = AtomicBool::new(false);

const GAMES_PER_THREAD: usize = 10_000;
const NODES_PER_GAME: u64 = 10_000;

fn main() {
    let mut handles = Vec::new();

    let book = File::open("resources/books/8moves_v3.epd").unwrap();

    let book_vec = BufReader::new(book).lines().map(|ln| ln.unwrap()).collect::<Vec<String>>();


    for seeds in book_vec.chunks(GAMES_PER_THREAD / 4).take(4) {
        let x = seeds.to_vec();
        handles.push(
            spawn(move || {
                let mut worker = ThreadData::new(NODES_PER_GAME, 8);
                worker.seed_positions = x;
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
