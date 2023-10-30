const HIDDEN: usize = 512;
const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;
const OUTPUT_BUCKETS: usize = 8;

#[inline]
fn activate(x: i16) -> i32 {
    i32::from(x.clamp(0, QA as i16))
}

#[repr(C)]
pub struct Network {
    feature_weights: [Accumulator; 768],
    feature_bias: Accumulator,
    output_weights: [[Accumulator; 2]; OUTPUT_BUCKETS],
    output_bias: [i16; OUTPUT_BUCKETS],
}

static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../resources/giuseppe.bin")) };

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator, occ: u64) -> i32 {
        let bucket = (occ.count_ones() - 2) as usize / 4;
        let mut sum = i32::from(NNUE.output_bias[bucket]);

        for (&x, &w) in boys.vals.iter().zip(&NNUE.output_weights[bucket][0].vals) {
            sum += activate(x) * i32::from(w);
        }

        for (&x, &w) in opps.vals.iter().zip(&NNUE.output_weights[bucket][1].vals) {
            sum += activate(x) * i32::from(w);
        }

        sum * SCALE / QAB
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HIDDEN],
}

impl Accumulator {
    pub fn update<const ADD: bool>(&mut self, idx: usize) {
        assert!(idx < 768);
        for (i, d) in self.vals.iter_mut().zip(&NNUE.feature_weights[idx].vals) {
            if ADD {
                *i += *d
            } else {
                *i -= *d
            }
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        NNUE.feature_bias
    }
}
