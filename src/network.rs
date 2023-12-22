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

fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
    #[cfg(not(target_feature="avx2"))]
    {
        #[inline]
        fn screlu(x: i16) -> i32 {
            i32::from(x.clamp(0, QA as i16)).pow(2)
        }

        let mut sum = 0;

        for (&x, &w) in acc.vals.iter().zip(&weights.vals) {
            sum += screlu(x) * i32::from(w);
        }

        sum
    }

    #[cfg(target_feature="avx2")]
    unsafe {
        use std::arch::x86_64::*;

        const CHUNK: usize = 8;

        let mut sum = _mm256_setzero_si256();
        let min = _mm256_setzero_si256();
        let max = _mm256_set1_epi32(QA);

        for i in 0..HIDDEN / CHUNK {
            let mut v = _mm256_cvtepi16_epi32(_mm_load_si128(acc.vals.as_ptr().add(i * CHUNK).cast()));
            v = _mm256_min_epi32(_mm256_max_epi32(v, min), max);
            v = _mm256_mullo_epi32(v, v);

            let w = _mm256_cvtepi16_epi32(_mm_load_si128(weights.vals.as_ptr().add(i * CHUNK).cast()));

            let product = _mm256_mullo_epi32(v, w);

            sum = _mm256_add_epi32(sum, product);
        }

        let mut res = _mm256_extract_epi32::<0>(sum);
        res += _mm256_extract_epi32::<1>(sum);
        res += _mm256_extract_epi32::<2>(sum);
        res += _mm256_extract_epi32::<3>(sum);
        res += _mm256_extract_epi32::<4>(sum);
        res += _mm256_extract_epi32::<5>(sum);
        res += _mm256_extract_epi32::<6>(sum);
        res += _mm256_extract_epi32::<7>(sum);
        res
        /*
        let upper_128 = _mm256_extracti128_si256::<1>(sum);
        let lower_128 = _mm256_castsi256_si128(sum);
        let sum_128 = _mm_add_epi64(upper_128, lower_128);
        let upper_64 = _mm_unpackhi_epi64(sum_128, sum_128);
        let sum_64 = _mm_add_epi64(upper_64, sum_128);
        let upper_32 = _mm_shuffle_epi32::<0b10_11_00_01>(sum_64);
        let sum_32 = _mm_add_epi32(upper_32, sum_64);

        _mm_cvtsi128_si32(sum_32)*/
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
