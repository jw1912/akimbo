use crate::util::boxed_and_zeroed;

const HIDDEN: usize = 1024;
const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[repr(C)]
pub struct Network {
    feature_weights: [Accumulator; 768 * NUM_BUCKETS],
    feature_bias: Accumulator,
    output_weights: [Accumulator; 2],
    output_bias: i16,
}

static NNUE: Network =
    unsafe { std::mem::transmute(*include_bytes!(concat!("../resources/net.bin"))) };

const NUM_BUCKETS: usize = 4;

#[rustfmt::skip]
static BUCKETS: [usize; 64] = [
    0, 0, 1, 1, 5, 5, 4, 4,
    2, 2, 2, 2, 6, 6, 6, 6,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
];

fn activ_dot_prod(acc: &Accumulator, weights: &Accumulator) -> i32 {
    let mut sum = 0;

    for (&x, &w) in acc.vals.iter().zip(&weights.vals) {
        // Lizard trick
        let v = x.clamp(0, QA as i16);
        let vw = v * w;
        sum += i32::from(v) * i32::from(vw);
    }

    sum
}

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator) -> i32 {
        let weights = &NNUE.output_weights;
        let sum = activ_dot_prod(boys, &weights[0]) + activ_dot_prod(opps, &weights[1]);
        (sum / QA + i32::from(NNUE.output_bias)) * SCALE / QAB
    }

    pub fn get_bucket<const SIDE: usize>(mut ksq: usize) -> usize {
        if SIDE == 1 {
            ksq ^= 56;
        }

        BUCKETS[ksq]
    }

    pub fn get_base_index<const SIDE: usize>(side: usize, pc: usize, mut ksq: usize) -> usize {
        if ksq % 8 > 3 {
            ksq ^= 7;
        }

        if SIDE == 0 {
            768 * Self::get_bucket::<0>(ksq) + [0, 384][side] + 64 * pc
        } else {
            768 * Self::get_bucket::<1>(ksq) + [384, 0][side] + 64 * pc
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HIDDEN],
}

impl Accumulator {
    pub fn update_multi(&mut self, adds: &[u16], subs: &[u16]) {
        const REGS: usize = 8;
        const PER: usize = REGS * 16;

        let mut regs = [0i16; PER];

        for i in 0..HIDDEN / PER {
            let offset = PER * i;

            for (j, reg) in regs.iter_mut().enumerate() {
                *reg = self.vals[offset + j];
            }

            for &add in adds {
                let weights = &NNUE.feature_weights[usize::from(add)];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg += weights.vals[offset + j];
                }
            }

            for &sub in subs {
                let weights = &NNUE.feature_weights[usize::from(sub)];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg -= weights.vals[offset + j];
                }
            }

            for (j, reg) in regs.iter().enumerate() {
                self.vals[offset + j] = *reg;
            }
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        NNUE.feature_bias
    }
}

pub struct EvalEntry {
    pub bbs: [u64; 8],
    pub white: Accumulator,
    pub black: Accumulator,
}

pub struct EvalTable {
    pub table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2 * NUM_BUCKETS]>,
}

impl Default for EvalTable {
    fn default() -> Self {
        let mut table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2 * NUM_BUCKETS]> = unsafe { boxed_and_zeroed() };

        for row in table.iter_mut() {
            for entry in row.iter_mut() {
                entry.white = Accumulator::default();
                entry.black = Accumulator::default();
            }
        }

        Self { table }
    }
}
