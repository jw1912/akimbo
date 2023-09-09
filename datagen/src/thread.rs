use akimbo::{position::{Position, Move}, search::{Engine, go}};
use crate::{util::{to_fen, is_capture, is_terminal}, STOP};
use std::{time::{SystemTime, UNIX_EPOCH, Instant}, io::{BufWriter, Write}, fs::File, sync::atomic::Ordering};

#[derive(Default)]
pub struct GameResult {
    fens: Vec<String>,
    result: f32,
}

pub struct ThreadData {
    engine: Engine,
    id: u64,
    rng: u64,
    file: BufWriter<File>,
    games: u64,
    fens: u64,
    start_time: Instant,
    nodes_per_move: u64,
}

impl ThreadData {
    pub fn show_status(&self) {
        let fps = self.fens as f64 / self.start_time.elapsed().as_secs_f64();
        let fpg = self.fens / self.games;
        println!("thread id {} games {} fens {} fps {fps:.0} fpg {fpg}", self.id, self.games, self.fens);
    }

    pub fn new(nodes_per_move: u64, hash_size: usize) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Guaranteed increasing.")
            .as_micros() as u64 & 0xFFFF_FFFF;

        let mut res = Self {
            engine: Engine {
                mloop: false,
                max_nodes: 1_000_000,
                max_time: 10000,
                ..Default::default()
            },
            id: seed,
            rng: seed,
            file: BufWriter::new(File::create(format!("resources/akimbo-{seed}.epd")).unwrap()),
            games: 0,
            fens: 0,
            start_time: Instant::now(),
            nodes_per_move,
        };

        res.engine.resize_tt(hash_size);

        println!("thread id {} created", res.id);
        res
    }

    pub fn write(&mut self, result: GameResult) {
        self.games += 1;
        let num_taken = result.fens.len().saturating_sub(if result.result == 0.5 {8} else {0});
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

    pub fn reset(&mut self) {
        self.engine.clear_tt();
        self.engine.htable = Box::new([[[(0, Move::NULL); 64]; 8]; 2]);
    }

    pub fn run_datagen(&mut self, max_games: u64) {
        while self.games < max_games {
            if STOP.load(Ordering::SeqCst) {
                break;
            }

            let result = if let Some(res) = self.run_game() {
                res
            } else {
                continue;
            };

            self.write(result);
            if self.games % 20 == 0 {
                self.show_status();
            }
        }
        self.file.flush().unwrap();
        println!("thread id {} finished", self.id);
        self.show_status();
    }

    pub fn run_game(&mut self) -> Option<GameResult>{
        self.reset();

        let mut position;

        position = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        // play 8 or 9 random moves
        for _ in 0..(8 + (self.rng() % 2)) {
            let moves = position.movegen::<true>();
            let mut legals = Vec::new();
            for &mov in &moves.list[..moves.len] {
                let mut new = position;
                if !new.make(mov) {
                    legals.push(mov);
                }
            }

            if legals.is_empty() {
                return None;
            }

            self.engine.stack.push(position.hash());
            position.make(legals[self.rng() as usize % legals.len()]);
        }

        let mut result = GameResult::default();

        // play out game
        loop {
            let (bm, score) = go(&position, &mut self.engine, false, 32, 1000.0, self.nodes_per_move);

            // adjudicate large scores
            if score.abs() > 1000 {
                result.result = f32::from(if score > 0 {!position.c} else {position.c});
                break;
            }

            // position is quiet, can use fen
            if !is_capture(bm) && !position.in_check() {
                result.fens.push(to_fen(&position, score));
            }

            // not enough nodes to finish a depth!
            self.engine.stack.push(position.hash());
            if bm == Move::NULL || position.make(bm) {
                return None
            }

            // check for game end via check/stalemate
            if is_terminal(&position) {
                result.result = if position.in_check() {
                    f32::from(position.c)
                } else {
                    0.5
                };
                break;
            }

            // check for game end via other draw rules
            if position.draw() || self.engine.repetition(&position, position.hash(), true) {
                result.result = 0.5;
                break;
            }
        }

        Some(result)
    }
}
