use akimbo::{position::Position, search::{Engine, go}};
use crate::util::{to_fen, is_capture, is_terminal};
use std::{time::{SystemTime, UNIX_EPOCH, Instant}, io::{BufWriter, Write}, fs::File};

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
}

impl ThreadData {
    pub fn show_status(&self) {
        let fps = self.fens as f64 / self.start_time.elapsed().as_secs_f64();
        println!("id {} games {} fens {} fps {fps:.0}", self.id, self.games, self.fens);
    }

    pub fn new(max_nodes: u64, hash_size: usize) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        let mut res = Self {
            engine: Engine {
                max_nodes,
                max_time: 10000,
                ..Default::default()
            },
            id: seed,
            rng: seed,
            file: BufWriter::new(File::create(format!("resources/akimbo-{seed}.epd")).unwrap()),
            games: 0,
            fens: 0,
            start_time: Instant::now(),
        };

        res.engine.resize_tt(hash_size);
        res
    }

    pub fn write(&mut self, result: GameResult) {
        self.games += 1;
        for fen in result.fens.iter().take(result.fens.len().saturating_sub(8)) {
            writeln!(&mut self.file, "{} [{:.1}]", fen, result.result).unwrap();
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
        self.engine.htable = Box::new([[[Default::default(); 64]; 8]; 2]);
    }

    pub fn run_datagen(&mut self, max_games: u64) {
        for _ in 0..max_games {
            let result = self.run_game().unwrap();
            self.write(result);
            if self.games % 20 == 0 {
                self.show_status();
            }
        }
        self.file.flush().unwrap();
        println!("id {} finished", self.id);
        self.show_status();
    }

    pub fn run_game(&mut self) -> Option<GameResult>{
        self.reset();

        let mut position = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        // play 8 random moves
        for _ in 0..8 {
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

            position.make(legals[self.rng() as usize % legals.len()]);
        }

        let mut result = GameResult::default();

        // play out game
        loop {
            let bm = go(&position, &mut self.engine, false, 10, 1000.0);

            // position is quiet, can use fen
            if !is_capture(bm) && !position.in_check() {
                result.fens.push(to_fen(&position));
            }

            position.make(bm);

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
