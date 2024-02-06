const HIDDEN: usize = 768;
const SCALE: i32 = 400;
const QA: i32 = 181;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[repr(C)]
pub struct Network {
    feature_weights: [Accumulator; 768 * NUM_BUCKETS],
    feature_bias: Accumulator,
    output_weights: [Accumulator; 2],
    output_bias: i16,
}

static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../resources/net-06.02.24-epoch17.bin")) };

const NUM_BUCKETS: usize = 4;
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

    #[allow(clippy::modulo_one)]
    pub fn bucket(sq: u8) -> usize {
        BUCKETS[usize::from(sq)]
    }
}

#[derive(Clone, Copy, Default)]
pub struct FeatureBuffer {
    adds: [(u16, u16); 2],
    subs: [(u16, u16); 2],
    add_count: usize,
    sub_count: usize,
    needs_refresh: bool,
}

impl FeatureBuffer {
    pub fn clear(&mut self) {
        self.needs_refresh = false;
        self.add_count = 0;
        self.sub_count = 0;
    }

    pub fn must_refresh(&mut self) {
        self.needs_refresh = true;
    }

    pub fn needs_refresh(&self) -> bool {
        self.needs_refresh
    }

    pub fn push_add(&mut self, wfeat: usize, bfeat: usize) {
        self.adds[self.add_count] = (wfeat as u16, bfeat as u16);
        self.add_count += 1;
    }

    pub fn push_sub(&mut self, wfeat: usize, bfeat: usize) {
        self.subs[self.sub_count] = (wfeat as u16, bfeat as u16);
        self.sub_count += 1;
    }

    pub fn update_accumulators(&self, accs: &mut [Accumulator; 2]) {
        for &(wfeat, bfeat) in self.adds.iter().take(self.add_count) {
            accs[0].update::<true>(usize::from(wfeat));
            accs[1].update::<true>(usize::from(bfeat));
        }

        for &(wfeat, bfeat) in self.subs.iter().take(self.sub_count) {
            accs[0].update::<false>(usize::from(wfeat));
            accs[1].update::<false>(usize::from(bfeat));
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HIDDEN],
}

impl Accumulator {
    pub fn update<const ADD: bool>(&mut self, idx: usize) {
        assert!(idx < 768 * NUM_BUCKETS);
        for (i, d) in self.vals.iter_mut().zip(&NNUE.feature_weights[idx].vals) {
            if ADD {
                *i += *d
            } else {
                *i -= *d
            }
        }
    }

    pub fn get_white_index(side: usize, pc: usize, mut sq: usize, mut ksq: u8) -> usize {
        if ksq % 8 > 3 {
            sq ^= 7;
            ksq ^= 7;
        }
        768 * Network::bucket(ksq) + [0, 384][side] + 64 * pc + sq
    }

    pub fn get_black_index(side: usize, pc: usize, mut sq: usize, mut ksq: u8) -> usize {
        ksq ^= 56;
        if ksq % 8 > 3 {
            sq ^= 7;
            ksq ^= 7;
        }
        768 * Network::bucket(ksq) + [384, 0][side] + 64 * pc + (sq ^ 56)
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

        const CHUNK: usize = 16;

        let mut sum = _mm256_setzero_si256();

        for i in 0..HIDDEN / CHUNK {
            let v = screlu(load_i16s(acc, i * CHUNK));
            let w = load_i16s(weights, i * CHUNK);
            let product = _mm256_madd_epi16(v, w);
            sum = _mm256_add_epi32(sum, product);
        }

        horizontal_sum_i32(sum)
    }

    #[inline]
    unsafe fn screlu(mut v: __m256i) -> __m256i {
        let min = _mm256_setzero_si256();
        let max = _mm256_set1_epi16(QA as i16);
        v = _mm256_min_epi16(_mm256_max_epi16(v, min), max);
        _mm256_mullo_epi16(v, v)
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
