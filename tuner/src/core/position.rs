use crate::{bitloop, core::{OPEN, SEMI}};

use super::{Params, S, OFFSET, PASSER};
use std::str::FromStr;

pub const TPHASE: f64 = 24.0;

#[derive(Default)]
pub struct Position {
    pub indices: [[u16; 16]; 2],
    pub passers: [u64; 2],
    pub opens: [u64; 2],
    pub semis: [u64; 2],
    pub counters: [u8; 2],
    pub offsets: [u16; 2],
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
        let mut pawns = [0, 0];
        let mut rooks = [0, 0];
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
                    pos.offsets[c] = 5 * 64 * (sq ^ flip);
                } else {
                    pos.indices[c][pos.counters[c] as usize] = pc * 64 + (sq ^ flip);
                    pos.counters[c] += 1;
                    pos.phase += [0., 1., 1., 2., 4., 0.][pc as usize];
                }

                // get necessary bitboards
                if pc == 0 {
                    pawns[c] |= 1 << sq;
                } else if pc == 3 {
                    rooks[c] |= 1 << sq;
                }

                col += 1;
            }
        }

        pos.passers[0] = passers(pawns[0], pawns[1]);
        pos.passers[1] = passers(pawns[1].swap_bytes(), pawns[0].swap_bytes());

        let wopen = !full_spans(pawns[0]);
        let bopen = !full_spans(pawns[1]);
        pos.opens[0] = rooks[0] & wopen & bopen;
        pos.opens[1] = rooks[1] & wopen & bopen;
        pos.semis[0] = rooks[0] & wopen;
        pos.semis[1] = rooks[1] & bopen;

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

fn passers(boys: u64, opps: u64) -> u64 {
    use akimbo::util::File;
    let mut spans = (!File::A & opps) >> 1 | opps | (!File::H & opps) << 1;
    spans >>= 8;
    spans |= spans >> 8;
    spans |= spans >> 16;
    spans |= spans >> 32;
    boys & !spans
}

fn full_spans(mut bb: u64) -> u64 {
    bb |= bb >> 8;
    bb |= bb >> 16;
    bb |= bb >> 32;
    bb |= bb << 8;
    bb |= bb << 16;
    bb |= bb << 32;
    bb
}

impl Position {
    pub fn eval(&self, params: &Params) -> f64 {
        let mut score = S::new(0.);
        for i in 0..usize::from(self.counters[0]) {
            let idx = self.indices[0][i];
            score += params[self.offsets[0] + idx] + params[OFFSET as u16 + self.offsets[1] + idx];
        }
        for i in 0..usize::from(self.counters[1]) {
            let idx = self.indices[1][i];
            score -= params[self.offsets[1] + idx] + params[OFFSET as u16 + self.offsets[0] + idx];
        }

        bitloop!(self.passers[0], sq, score += params[PASSER as u16 + sq]);
        bitloop!(self.passers[1], sq, score -= params[PASSER as u16 + sq]);
        bitloop!(self.opens[0], sq, score += params[OPEN as u16 + (sq & 7)]);
        bitloop!(self.opens[1], sq, score -= params[OPEN as u16 + (sq & 7)]);
        bitloop!(self.semis[0], sq, score += params[SEMI as u16 + (sq & 7)]);
        bitloop!(self.semis[1], sq, score -= params[SEMI as u16 + (sq & 7)]);

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
