use crate::util::boxed_and_zeroed;

const HIDDEN: usize = 768;
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
    unsafe { std::mem::transmute(*include_bytes!(concat!("../", env!("EVALFILE")))) };

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

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator) -> i32 {
        let weights = &NNUE.output_weights;
        let sum = flatten(boys, &weights[0]) + flatten(opps, &weights[1]);
        (sum / QA + i32::from(NNUE.output_bias)) * SCALE / QAB
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
    pub fn update_multi(
        &mut self,
        adds: &[usize],
        subs: &[usize],
    ) {
        #[cfg(not(target_feature = "avx2"))]
        const REGS: usize = 8;
        #[cfg(target_feature = "avx2")]
        const REGS: usize = 16;

        const PER: usize = REGS * 16;

        let mut regs = [0i16; PER];

        for i in 0..HIDDEN / PER {
            let offset = PER * i;

            for (j, reg) in regs.iter_mut().enumerate() {
                *reg = self.vals[offset + j];
            }

            for &add in adds {
                let weights = &NNUE.feature_weights[add];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg += weights.vals[offset + j];
                }
            }

            for &sub in subs {
                let weights = &NNUE.feature_weights[sub];

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
    pub acc: Accumulator,
}

pub struct EvalTable {
    pub table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2]>,
}

impl Default for EvalTable {
    fn default() -> Self {
        let mut table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2]> = boxed_and_zeroed();

        for side in table.iter_mut() {
            for entry in side.iter_mut() {
                entry.acc = Accumulator::default();
            }
        }

        Self { table }
    }
}

fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
    #[cfg(not(target_feature = "avx2"))]
    {
        fallback::flatten(acc, weights)
    }
    #[cfg(target_feature = "avx2")]
    unsafe {
        avx2::flatten(acc, weights)
    }
}

#[cfg(not(target_feature = "avx2"))]
mod fallback {
    use super::{Accumulator, QA};

    #[inline]
    pub fn screlu(x: i16) -> i32 {
        i32::from(x.clamp(0, QA as i16)).pow(2)
    }

    #[inline]
    pub fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
        let mut sum = 0;

        for (&x, &w) in acc.vals.iter().zip(&weights.vals) {
            sum += screlu(x) * i32::from(w);
        }

        sum
    }
}

#[cfg(target_feature = "avx2")]
mod avx2 {
    use super::{Accumulator, HIDDEN, QA};
    use std::arch::x86_64::*;

    pub unsafe fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
        use std::arch::x86_64::*;

        const CHUNK: usize = 16;

        let mut sum = _mm256_setzero_si256();
        let min = _mm256_setzero_si256();
        let max = _mm256_set1_epi16(QA as i16);

        for i in 0..HIDDEN / CHUNK {
            let mut v = load_i16s(acc, i * CHUNK);
            v = _mm256_min_epi16(_mm256_max_epi16(v, min), max);
            let w = load_i16s(weights, i * CHUNK);
            let product = _mm256_madd_epi16(v, _mm256_mullo_epi16(v, w));
            sum = _mm256_add_epi32(sum, product);
        }

        horizontal_sum_i32(sum)
    }

    #[inline]
    unsafe fn load_i16s(acc: &Accumulator, start_idx: usize) -> __m256i {
        _mm256_load_si256(acc.vals.as_ptr().add(start_idx).cast())
    }

    #[inline]
    unsafe fn horizontal_sum_i32(sum: __m256i) -> i32 {
        let upper_128 = _mm256_extracti128_si256::<1>(sum);
        let lower_128 = _mm256_castsi256_si128(sum);
        let sum_128 = _mm_add_epi32(upper_128, lower_128);
        let upper_64 = _mm_unpackhi_epi64(sum_128, sum_128);
        let sum_64 = _mm_add_epi32(upper_64, sum_128);
        let upper_32 = _mm_shuffle_epi32::<0b00_00_00_01>(sum_64);
        let sum_32 = _mm_add_epi32(upper_32, sum_64);

        _mm_cvtsi128_si32(sum_32)
    }
}
