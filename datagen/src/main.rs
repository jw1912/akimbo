mod util;
mod thread;

use thread::ThreadData;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    sync::atomic::{AtomicBool, Ordering},
    thread::{spawn, sleep}
};

pub static STOP: AtomicBool = AtomicBool::new(false);

const GAMES_PER_THREAD: usize = 20_000;
const NODES_PER_GAME: u64 = 40_000;

fn main() {
    let mut handles = Vec::new();

    let noob8 = File::open("resources/books/8moves_v3.epd").unwrap();

    let noob8_vec = BufReader::new(noob8).lines().map(|ln| ln.unwrap()).take(GAMES_PER_THREAD).collect::<Vec<String>>();

    sleep(std::time::Duration::from_millis(500));
    handles.push(
        spawn(move || {
            let mut worker = ThreadData::new(NODES_PER_GAME, 8);
            worker.seed_positions = noob8_vec;
            worker.run_datagen(GAMES_PER_THREAD as u64);
        })
    );

    let pohl = File::open("resources/books/Pohl.epd").unwrap();
    let pohl_vec = BufReader::new(pohl).lines().map(|ln| ln.unwrap()).take(2 * GAMES_PER_THREAD).collect::<Vec<String>>();

    for seeds in pohl_vec.chunks(GAMES_PER_THREAD / 2).take(3) {
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
