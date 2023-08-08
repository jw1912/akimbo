use crate::{core::*, bitloop};
use std::{fs::File, io::{BufRead, BufReader, BufWriter, Write}, thread, time::Instant};

#[derive(Default)]
pub struct Data(Vec<Position>, pub usize, pub u64);

impl Data {
    pub fn num(&self) -> f64 {
        self.0.len() as f64
    }

    pub fn rng(&mut self) -> u64 {
        self.2 ^= self.2 << 13;
        self.2 ^= self.2 >> 7;
        self.2 ^= self.2 << 17;
        self.2
    }

    pub fn add_contents<const FILTER: bool>(&mut self, file_name: &str) {
        self.2 = 234232423;
        let timer = Instant::now();
        let (mut wins, mut losses, mut draws) = (0, 0, 0);
        let file = File::open(file_name).unwrap();
        let mut used = BufWriter::new(File::create("resources/used.epd").unwrap());
        for line in BufReader::new(file).lines().map(|ln| ln.unwrap()) {
            let res: Position = line.parse().unwrap();
            let int = (res.result * 2.0) as u64;
            if FILTER {
                if int == 1 && self.rng() % 2 == 1 {
                    continue;
                }
                writeln!(&mut used, "{}", line).unwrap();
            }
            match int {
                2 => wins += 1,
                0 => losses += 1,
                1 => draws += 1,
                _ => unreachable!(),
            }
            self.0.push(res);
        }
        let elapsed = timer.elapsed().as_secs_f64();
        let pps = self.num() / elapsed;
        println!("{} positions in {elapsed:.2} seconds, {pps:.2} pos/sec", self.num());
        println!("wins {wins} losses {losses} draws {draws}");
    }

    pub fn error(&self, k: f64, params: &Params) -> f64 {
        let size = self.0.len() / self.1;
        thread::scope(|s| {
            self.0
                .chunks(size)
                .map(|chunk| s.spawn(|| chunk.iter().map(|p| p.err(k, params)).sum::<f64>()))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|p| p.join().unwrap_or_default())
                .sum::<f64>()
        }) / self.num()
    }

    pub fn gradients(&self, k: f64, params: &Params) -> Params {
        let size = self.0.len() / self.1;
        thread::scope(|s| {
            self.0
                .chunks(size)
                .map(|chunk| s.spawn(|| gradients_batch(chunk, k, params)))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|p| p.join().unwrap_or_default())
                .fold(Params::default(), |a, b| a + b)
        })
    }
}

fn gradients_batch(positions: &[Position], k: f64, params: &Params) -> Params {
    let mut grad = Params::default();
    for pos in positions {
        let sigm = sigmoid(k * pos.eval(params));
        let term = (pos.result - sigm) * (1. - sigm) * sigm;
        let phase_adj = term *  S(pos.phase, 1. - pos.phase);

        for i in 0..usize::from(pos.counters[0]) {
            let idx = pos.indices[0][i];
            grad[pos.offsets[0] + idx] += phase_adj;
            grad[OFFSET as u16 + pos.offsets[1] + idx] += phase_adj;
        }
        for i in 0..usize::from(pos.counters[1]) {
            let idx = pos.indices[1][i];
            grad[pos.offsets[1] + idx] -= phase_adj;
            grad[OFFSET as u16 + pos.offsets[0] + idx] -= phase_adj;
        }

        bitloop!(pos.passers[0], sq, grad[PASSER as u16 + sq] += phase_adj);
        bitloop!(pos.passers[1], sq, grad[PASSER as u16 + sq] -= phase_adj);
        bitloop!(pos.opens[0], sq, grad[OPEN as u16 + (sq & 7)] += phase_adj);
        bitloop!(pos.opens[1], sq, grad[OPEN as u16 + (sq & 7)] -= phase_adj);
        bitloop!(pos.semis[0], sq, grad[SEMI as u16 + (sq & 7)] += phase_adj);
        bitloop!(pos.semis[1], sq, grad[SEMI as u16 + (sq & 7)] -= phase_adj);
    }
    grad
}
