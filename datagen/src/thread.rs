use akimbo::{
    moves::Move,
    position::Position,
    search::go,
    tables::{HashTable, HistoryTable},
    thread::ThreadData,
};

use crate::{
    util::{is_terminal, to_fen, update_display},
    STOP,
};

use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct GameResult {
    fens: Vec<String>,
    result: f32,
}

pub struct DatagenThread {
    hash_size: usize,
    id: u64,
    rng: u64,
    file: BufWriter<File>,
    games: u64,
    fens: u64,
    start_time: Instant,
    nodes_per_move: u64,
}

impl DatagenThread {
    pub fn new(nodes_per_move: u64, hash_size: usize) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Guaranteed increasing.")
            .as_micros() as u64
            & 0xFFFF_FFFF;

        let res = Self {
            hash_size,
            id: seed,
            rng: seed,
            file: BufWriter::new(File::create(format!("resources/akimbo-{seed}.epd")).unwrap()),
            games: 0,
            fens: 0,
            start_time: Instant::now(),
            nodes_per_move,
        };

        println!("thread id {} created", res.id);
        res
    }

    pub fn write(&mut self, result: GameResult) {
        self.games += 1;
        let num_taken = result
            .fens
            .len()
            .saturating_sub(if result.result == 0.5 { 8 } else { 0 });
        for fen in result.fens.iter().take(num_taken) {
            writeln!(&mut self.file, "{} | {:.1}", fen, result.result).unwrap();
            self.fens += 1;
        }
    }

    pub fn rng(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn update_display(&self, num: usize, games: &[AtomicU64], fens: &[AtomicU64]) {
        games[num].store(self.games, Ordering::Relaxed);
        fens[num].store(self.fens, Ordering::Relaxed);
        update_display(self.start_time, games, fens);
    }

    pub fn run_datagen(
        &mut self,
        max_games: u64,
        num: usize,
        games: &[AtomicU64],
        fens: &[AtomicU64],
        startpos: Position,
    ) {
        let mut tt = HashTable::default();
        tt.resize(self.hash_size, 1);

        while self.games < max_games {
            if STOP.load(Ordering::SeqCst) {
                break;
            }

            let optional = self.run_game(&tt, startpos);
            tt.clear(1);

            let result = if let Some(res) = optional {
                res
            } else {
                continue;
            };

            self.write(result);
            if self.games % 10 == 0 {
                self.update_display(num, games, fens);
            }
        }
        self.file.flush().unwrap();
    }

    pub fn run_game(&mut self, tt: &HashTable, mut position: Position) -> Option<GameResult> {
        let abort = AtomicBool::new(false);
        let mut engine = ThreadData {
            mloop: false,
            max_nodes: 1_000_000,
            max_time: 10000,
            ..ThreadData::new(&abort, tt, Vec::new(), HistoryTable::default())
        };

        // play 8 or 9 random moves
        for _ in 0..(8 + (self.rng() % 2)) {
            let moves = position.movegen::<true>();
            let mut legals = Vec::new();
            for &mov in moves.iter() {
                let mut new = position;
                if !new.make(mov) {
                    legals.push(mov);
                }
            }

            if legals.is_empty() {
                return None;
            }

            engine.stack.push(position.hash());
            position.make(legals[self.rng() as usize % legals.len()]);
        }

        let mut result = GameResult::default();

        // play out game
        loop {
            let (bm, score) = go(
                &position,
                &mut engine,
                false,
                32,
                1000.0,
                self.nodes_per_move,
            );

            // adjudicate large scores
            if score.abs() > 1000 {
                result.result = if score > 0 {
                    1 - position.stm()
                } else {
                    position.stm()
                } as f32;

                break;
            }

            // position is quiet, can use fen
            if !bm.is_capture() && !position.in_check() {
                result.fens.push(to_fen(&position, score));
            }

            // not enough nodes to finish a depth!
            engine.stack.push(position.hash());
            if bm == Move::NULL || position.make(bm) {
                return None;
            }

            // check for game end via check/stalemate
            if is_terminal(&position) {
                result.result = if position.in_check() {
                    position.stm() as f32
                } else {
                    0.5
                };
                break;
            }

            // check for game end via other draw rules
            if position.draw() || engine.repetition(&position, position.hash(), true) {
                result.result = 0.5;
                break;
            }
        }

        Some(result)
    }
}
