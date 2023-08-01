use crate::core::{Params, Position, S, sigmoid};
use std::{fs::File, io::{BufRead, BufReader}, thread};

#[derive(Default)]
pub struct Data(Vec<Position>, pub usize);

impl Data {
    pub fn num(&self) -> f64 {
        self.0.len() as f64
    }

    pub fn add_contents(&mut self, file_name: &str) -> f64 {
        let file = File::open(file_name).unwrap();
        for line in BufReader::new(file).lines().map(|ln| ln.unwrap()) {
            self.0.push(line.parse().unwrap());
        }
        self.num()
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
            grad[idx] += phase_adj;
        }
        for i in 0..usize::from(pos.counters[1]) {
            let idx = pos.indices[1][i];
            grad[idx] -= phase_adj;
        }
    }
    grad
}
