mod attacks;
mod indices;
mod input;
mod offsets;
mod threats;

use crate::position::Position;

const HIDDEN: usize = 2048;
const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[repr(C)]
pub struct Network {
    feature_weights: [Accumulator; input::TOTAL],
    feature_bias: Accumulator,
    output_weights: [Accumulator; 2],
    output_bias: i16,
}

static NNUE: Network =
    unsafe { std::mem::transmute(*include_bytes!(concat!("../resources/net.bin"))) };

impl Network {
    pub fn out(pos: &Position) -> i32 {
        let mut white = Accumulator::default();
        let mut black = Accumulator::default();

        let mut bbs = pos.bbs();

        let mut count = 0;
        let mut feats = [0; 128];
        input::map_features_single(bbs, |stm| {feats[count] = stm as u32; count += 1;});
        white.update_multi(&feats[..count]);
    
        bbs.swap(0, 1);
        for bb in &mut bbs {
            *bb = bb.swap_bytes();
        }
    
        count = 0;
        input::map_features_single(bbs, |ntm| {feats[count] = ntm as u32; count += 1;});
        black.update_multi(&feats[..count]);

        let weights = &NNUE.output_weights;
        let sum = flatten(&white, &weights[pos.stm()]) + flatten(&black, &weights[1 - pos.stm()]);
        (sum / QA + i32::from(NNUE.output_bias)) * SCALE / QAB
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HIDDEN],
}

impl Accumulator {
    pub fn update_multi(&mut self, adds: &[u32]) {
        const REGS: usize = 8;
        const PER: usize = REGS * 16;

        let mut regs = [0i16; PER];

        for i in 0..HIDDEN / PER {
            let offset = PER * i;

            for (j, reg) in regs.iter_mut().enumerate() {
                *reg = self.vals[offset + j];
            }

            for &add in adds {
                let weights = &NNUE.feature_weights[add as usize];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg += weights.vals[offset + j];
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
