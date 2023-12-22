const HIDDEN: usize = 768;
const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;
const OUTPUT_BUCKETS: usize = 8;

#[repr(C)]
pub struct Network {
    feature_weights: [Accumulator; 768],
    feature_bias: Accumulator,
    output_weights: [[Accumulator; 2]; OUTPUT_BUCKETS],
    output_bias: [i16; OUTPUT_BUCKETS],
}

static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../resources/net-epoch30.bin")) };

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator, occ: u64) -> i32 {
        let bucket = (occ.count_ones() - 2) as usize / 4;
        let weights = &NNUE.output_weights[bucket];
        let sum = flatten(boys, &weights[0]) + flatten(opps, &weights[1]);
        (sum / QA + i32::from(NNUE.output_bias[bucket])) * SCALE / QAB
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

fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
    #[cfg(not(target_feature="avx2"))]
    {
        fallback::flatten(acc, weights)
    }
    #[cfg(target_feature="avx2")]
    unsafe {
        avx2::flatten(acc, weights)
    }
}

#[cfg(not(target_feature="avx2"))]
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

#[cfg(target_feature="avx2")]
mod avx2 {
    use std::arch::x86_64::*;
    use super::{Accumulator, QA, HIDDEN};

    pub unsafe fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
        use std::arch::x86_64::*;

        const CHUNK: usize = 8;

        let mut sum = _mm256_setzero_si256();

        for i in 0..HIDDEN / CHUNK {
            let v = screlu(load_and_extend_i32(acc, i * CHUNK));
            let w = load_and_extend_i32(weights, i * CHUNK);
            let product = _mm256_mullo_epi32(v, w);
            sum = _mm256_add_epi32(sum, product);
        }

        horizontal_sum_i32(sum)
    }

    #[inline]
    unsafe fn screlu(mut v: __m256i) -> __m256i {
        let min = _mm256_setzero_si256();
        let max = _mm256_set1_epi32(QA);
        v = _mm256_min_epi32(_mm256_max_epi32(v, min), max);
        _mm256_mullo_epi32(v, v)
    }

    #[inline]
    unsafe fn load_and_extend_i32(acc: &Accumulator, start_idx: usize) -> __m256i {
        _mm256_cvtepi16_epi32(_mm_load_si128(acc.vals.as_ptr().add(start_idx).cast()))
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
