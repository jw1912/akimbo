use akimbo::util::{Attacks, File};

use super::*;
use std::str::FromStr;

pub const NUM_PARAMS: usize = M_QUEEN + 28;

pub const OFFSET: usize = 5 * 64 * 64;
const PASSER: usize = 2 * OFFSET;
const OPEN: usize = PASSER + 64;
const SEMI: usize = OPEN + 8;
const BLOCKED: usize = SEMI + 8;
const M_KNIGHT: usize = BLOCKED + 8;
const M_BISHOP: usize = M_KNIGHT + 9;
const M_ROOK: usize = M_BISHOP + 14;
const M_QUEEN: usize = M_ROOK + 15;

macro_rules! bitloop {($bb:expr, $sq:ident, $func:expr) => {
    let mut bb = $bb;
    while bb > 0 {
        let $sq = bb.trailing_zeros() as u16;
        bb &= bb - 1;
        $func;
    }
}}

pub const TPHASE: f64 = 24.0;

#[derive(Default)]
pub struct Position {
    pub indices: [[u16; 16]; 2],
    pub active: [Vec<u16>; 2],
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
        let mut bitboards = [[0; 6]; 2];
        let mut occs = [0, 0];
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

                let bb = 1 << sq;
                occs[c] |= bb;
                bitboards[c][pc as usize] |= bb;

                col += 1;
            }
        }

        let occ = occs[0] | occs[1];
        let wp = bitboards[0][0];
        let bp = bitboards[1][0];

        // passed pawns
        let wpass = passers(wp, bp);
        let bpass = passers(bp.swap_bytes(), wp.swap_bytes());
        bitloop!(wpass, sq, pos.active[0].push(PASSER as u16 + sq));
        bitloop!(bpass, sq, pos.active[1].push(PASSER as u16 + sq));

        // blocked passed pawns
        let wblock = wpass & (occ >> 8);
        let bblock = bpass & (occ.swap_bytes() >> 8);
        bitloop!(wblock, sq, pos.active[0].push(BLOCKED as u16 + (sq / 8)));
        bitloop!(bblock, sq, pos.active[1].push(BLOCKED as u16 + (sq / 8)));

        let wopen = !full_spans(wp);
        let bopen = !full_spans(bp);

        // open rooks
        let wopens = bitboards[0][3] & wopen & bopen;
        let bopens = bitboards[1][3] & wopen & bopen;
        bitloop!(wopens, sq, pos.active[0].push(OPEN as u16 + (sq % 8)));
        bitloop!(bopens, sq, pos.active[1].push(OPEN as u16 + (sq % 8)));

        // semi-open rooks
        let wsemis = bitboards[0][3] & wopen;
        let bsemis = bitboards[1][3] & bopen;
        bitloop!(wsemis, sq, pos.active[0].push(SEMI as u16 + (sq % 8)));
        bitloop!(bsemis, sq, pos.active[1].push(SEMI as u16 + (sq % 8)));

        // pawn attacks
        let wpatt = ((wp & !File::A) << 7) | ((wp & !File::H) << 9);
        let bpatt = ((bp & !File::A) >> 9) | ((bp & !File::H) >> 7);
        let patt = [wpatt, bpatt];

        // mobility
        const MOBILITY: [usize; 4] = [M_KNIGHT, M_BISHOP, M_ROOK, M_QUEEN];
        for (side, pieces) in bitboards.iter().enumerate() {
            let bocc = occ ^ pieces[2] ^ pieces[4];
            let rocc = occ ^ pieces[3] ^ pieces[4];
            let qocc = occ ^ pieces[2] ^ pieces[3] ^ pieces[4];
            for (pc, &pcs) in pieces.iter().skip(1).take(4).enumerate() {
                bitloop!(pcs, sq, {
                    let mut attacks = match pc {
                        0 => Attacks::KNIGHT[sq as usize],
                        1 => Attacks::bishop(sq as usize, bocc),
                        2 => Attacks::rook(sq as usize, rocc),
                        3 => Attacks::rook(sq as usize, qocc) | Attacks::bishop(sq as usize, qocc),
                        _ => unreachable!(),
                    };
                    attacks &= !patt[side ^ 1];
                    pos.active[side].push(MOBILITY[pc] as u16 + attacks.count_ones() as u16);
                });
            }
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

fn passers(boys: u64, opps: u64) -> u64 {
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

        for &idx in &self.active[0] {
            score += params[idx];
        }

        for &idx in &self.active[1] {
            score -= params[idx];
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
