use crate::{position::Position, util::SIDE};

const INPUT: usize = 768;
const HIDDEN: usize = 16;

const SCALE: i32 = 65536;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[repr(C)]
struct NNUEParams {
    feature_weights: [i16; INPUT * HIDDEN],
    feature_bias: [i16; HIDDEN],
    output_weights: [i16; HIDDEN],
    output_bias: i16,
}

static NNUE: NNUEParams = unsafe {std::mem::transmute(*include_bytes!("../../resources/maiden-100.bin"))};

#[derive(Clone, Copy)]
struct Accumulator([i16; HIDDEN]);
impl Accumulator {
    pub fn add_feature(&mut self, feature_idx: usize) {
        let start = feature_idx * HIDDEN;
        for (i, d) in self.0.iter_mut().zip(&NNUE.feature_weights[start..start + HIDDEN]) {
            *i += *d;
        }
    }
}


pub fn eval(pos: &Position) -> i32 {
    let mut acc = Accumulator(NNUE.feature_bias);

    for (side, &boys) in pos.bb.iter().take(2).enumerate() {
        for (pc, &pc_bb) in pos.bb.iter().skip(2).enumerate() {
            let bucket = 384 * side + 64 * pc;
            let mut pcs = boys & pc_bb;
            while pcs > 0 {
                acc.add_feature(bucket + pcs.trailing_zeros() as usize);
                pcs &= pcs - 1;
            }
        }
    }

    let mut sum = 0;
    for (&i, &w) in acc.0.iter().zip(&NNUE.output_weights) {
        sum += i32::from(i.clamp(-255, 255)) * i32::from(w);
    }

    let flatten = sum / QA;

    SIDE[usize::from(pos.c)] * (flatten + i32::from(NNUE.output_bias)) * SCALE / QAB
}