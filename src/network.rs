use crate::util::boxed_and_zeroed;

const HIDDEN: usize = 1024;
const QA: i16 = 255;

const L2: usize = 16;
const L3: usize = 32;
const OB: usize = 8;

#[repr(C)]
pub struct Network {
    feature_weights: [Accumulator; 768 * NUM_BUCKETS],
    feature_bias: Accumulator,
    l2w: [[[f32; HIDDEN]; L2]; OB],
    l2b: [[f32; L2]; OB],
    l3w: [[[f32; L2]; L3]; OB],
    l3b: [[f32; L3]; OB],
    l4w: [[f32; L3]; OB],
    l4b: [f32; OB],
}

static NNUE: Network =
    unsafe { std::mem::transmute(*include_bytes!(concat!("../resources/net.bin"))) };

const NUM_BUCKETS: usize = 13;

#[rustfmt::skip]
static BUCKETS: [usize; 64] = [
     0,  1,  2,  3, 16, 15, 14, 13,
     4,  5,  6,  7, 20, 19, 18, 17,
     8,  8,  9,  9, 22, 22, 21, 21,
    10, 10, 10, 10, 23, 23, 23, 23,
    10, 10, 10, 10, 23, 23, 23, 23,
    12, 12, 12, 12, 25, 25, 25, 25,
    12, 12, 12, 12, 25, 25, 25, 25,
    12, 12, 12, 12, 25, 25, 25, 25,
];

fn screlu(x: f32) -> f32 {
    x.clamp(0.0, 1.0).powi(2)
}

fn pairwise(acc: &Accumulator, i: usize) -> f32 {
    f32::from(acc.vals[i]) * f32::from(acc.vals[i + HIDDEN / 2]) / f32::from(QA).powi(2)
}

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator, bucket: usize) -> i32 {
        let mut l1 = [0.0; HIDDEN];

        for i in 0..HIDDEN / 2 {
            l1[i] = pairwise(boys, i).clamp(0.0, 1.0);
            l1[i + HIDDEN / 2] = pairwise(opps, i).clamp(0.0, 1.0);
        }

        let mut l2 = NNUE.l2b[bucket];

        for (i, out) in l2.iter_mut().enumerate() {
            for (j, &inp) in l1.iter().enumerate() {
                *out += inp * NNUE.l2w[bucket][i][j];
            }

            *out = screlu(*out);
        }

        let mut l3 = NNUE.l3b[bucket];

        for (i, out) in l3.iter_mut().enumerate() {
            for (j, &inp) in l2.iter().enumerate() {
                *out += inp * NNUE.l3w[bucket][i][j];
            }
        }

        let mut eval = NNUE.l4b[bucket];

        for (&i, &j) in NNUE.l4w[bucket].iter().zip(l3.iter()) {
            eval += i * screlu(j);
        }

        (eval * 200.0) as i32
    }

    pub fn get_bucket<const SIDE: usize>(mut ksq: u8) -> usize {
        if SIDE == 1 {
            ksq ^= 56;
        }

        BUCKETS[usize::from(ksq)]
    }

    pub fn get_base_index<const SIDE: usize>(side: usize, pc: usize, mut ksq: u8) -> usize {
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
        let mut table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2 * NUM_BUCKETS]> = boxed_and_zeroed();

        for row in table.iter_mut() {
            for entry in row.iter_mut() {
                entry.white = Accumulator::default();
                entry.black = Accumulator::default();
            }
        }

        Self { table }
    }
}
