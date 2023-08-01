use super::{Params, S};
use std::{str::FromStr, sync::atomic::{AtomicU64, Ordering::Relaxed}};

pub const TPHASE: f64 = 24.0;

#[allow(clippy::declare_interior_mutable_const)]
const BLAH: AtomicU64 = AtomicU64::new(0);
pub static HITS: [AtomicU64; 64] = [BLAH; 64];

#[derive(Default)]
pub struct Position {
    pub indices: [[u16; 16]; 2],
    pub counters: [u8; 2],
    pub phase: f64,
    pub result: f64,
}

const CHARS: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
impl FromStr for Position {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pos = Position::default();
        let mut row = 7;
        let mut col = 0;
        let mut ksqs = [0, 0];
        for ch in s.chars() {
            if ch == '/' {
                row -= 1;
                col = 0;
            } else if ch == ' ' {
                break;
            } else if ('1'..='8').contains(&ch) {
                col += ch.to_digit(10).expect("hard coded") as u16;
            } else if let Some(idx) = CHARS.iter().position(|&element| element == ch) {
                let c = idx / 6;
                let pc = idx as u16 - 6 * c as u16;
                let sq = 8 * row + col;
                let flip = 56 * c as u16;

                // king?
                if pc == 5 {
                    ksqs[c] = sq ^ flip;
                } else {
                    pos.indices[c][pos.counters[c] as usize] = pc * 64 + (sq ^ flip);
                    pos.counters[c] += 1;
                    pos.phase += [0., 1., 1., 2., 4., 0.][pc as usize];
                }

                col += 1;
            }
        }

        for sq in ksqs {
            HITS[usize::from(sq)].fetch_add(1, Relaxed);
        }

        for i in 0..16 {
            pos.indices[0][i] += 5 * 64 * ksqs[0];
            pos.indices[1][i] += 5 * 64 * ksqs[1];
        }

        if pos.phase > TPHASE { pos.phase = TPHASE }
        pos.phase /= TPHASE;
        pos.result = match &s[(s.len() - 6)..] {
            "\"1-0\";" | " [1.0]" => 1.0,
            "\"0-1\";" | " [0.0]" => 0.0,
            _ => 0.5,
        };

        Ok(pos)
    }
}

impl Position {
    pub fn eval(&self, params: &Params) -> f64 {
        let mut score = S::new(0.);
        for i in 0..usize::from(self.counters[0]) {
            score += params[self.indices[0][i]];
        }
        for i in 0..usize::from(self.counters[1]) {
            score -= params[self.indices[1][i]];
        }

        self.phase * score.0 + (1. - self.phase) * score.1
    }

    pub fn err(&self, k: f64, params: &Params) -> f64 {
        (self.result - sigmoid(k * self.eval(params))).powi(2)
    }
}

#[inline]
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + f64::exp(-x))
}
