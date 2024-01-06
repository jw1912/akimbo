const L1: usize = 768;
const L2: usize = 8;
const L3: usize = 16;
const SCALE: i32 = 400;
const QA: i32 = 256;

#[repr(C)]
pub struct Network {
    l1_w: [Accumulator; 768],
    l1_b: Accumulator,
    l2: Layer<{L1 * 2}, L2>,
    l3: Layer<L2, L3>,
    l4: Layer<L3, 1>,
}

pub struct Activation;
impl Activation {
    #[inline]
    fn input(x: f32) -> f32 {
        x.clamp(0.0, 1.0).powi(2)
    }

    #[inline]
    fn output(x: f32) -> f32 {
        x.clamp(0.0, 1.0)
    }
}

static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../resources/morelayers-8-crelu-16-crelu-epoch8.bin")) };

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator) -> i32 {
        let l2 = NNUE.l2.concat_out(boys, opps);
        let l3 = NNUE.l3.out(&l2);
        let l4 = NNUE.l4.out(&l3);
        let out = l4.vals[0] * SCALE as f32;
        out as i32
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Layer<const M: usize, const N: usize> {
    weights: [Column<N>; M],
    biases: Column<N>,
}

impl Layer<{L1 * 2}, L2> {
    fn concat_out(&self, boys: &Accumulator, opps: &Accumulator) -> Column<L2> {
        let mut out = self.biases;
        out.flatten(boys, &self.weights[..L1]);
        out.flatten(opps, &self.weights[L1..]);
        out
    }
}

impl<const M: usize, const N: usize> Layer<M, N> {
    fn out(&self, inp: &Column<M>) -> Column<N> {
        let mut out = self.biases;

        for (&mul, col) in inp.vals.iter().zip(self.weights.iter()) {
            let act = Activation::output(mul);
            out.madd_assign(act, col);
        }

        out
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Column<const N: usize> {
    vals: [f32; N],
}

impl<const N: usize> Default for Column<N> {
    fn default() -> Self {
        Self { vals: [0.0; N] }
    }
}

impl<const N: usize> Column<N> {
    fn flatten(&mut self, acc: &Accumulator, weights: &[Column<N>]) {
        for (&x, col) in acc.vals.iter().zip(weights.iter()) {
            let act = Activation::input(f32::from(x) / QA as f32);
            self.madd_assign(act, col);
        }
    }

    fn madd_assign(&mut self, mul: f32, rhs: &Column<N>) {
        for (i, &j) in self.vals.iter_mut().zip(rhs.vals.iter()) {
            *i += mul * j;
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Accumulator {
    vals: [i16; L1],
}

impl Accumulator {
    pub fn update<const ADD: bool>(&mut self, idx: usize) {
        assert!(idx < 768);
        for (i, d) in self.vals.iter_mut().zip(&NNUE.l1_w[idx].vals) {
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
        NNUE.l1_b
    }
}

#[cfg(feature = "quantise")]
#[test]
fn _quantise() {
    use std::fs::File;
    use std::io::Write;

    const L1_SIZE: usize = 769 * L1;
    const L2_SIZE: usize = (L1 * 2 + 1) * L2;
    const L3_SIZE: usize = (L2 + 1) * L3;
    const OUT_SIZE: usize = L3 + 1;
    const SIZE: usize = L1_SIZE + L2_SIZE + L3_SIZE + OUT_SIZE;

    static RAW_NET: [f32; SIZE] = unsafe {
        std::mem::transmute(*include_bytes!("../../bullet/checkpoints/morelayers-8-crelu-16-crelu-epoch8/params.bin"))
    };

    let mut file = File::create("resources/morelayers-8-crelu-16-crelu-epoch8.bin").unwrap();

    fn write_buf<T>(buf: &[T], file: &mut File) {
        unsafe {
            let ptr = buf.as_ptr().cast();
            let size = std::mem::size_of_val(buf);
            let slice = std::slice::from_raw_parts(ptr, size);
            file.write_all(slice).unwrap();
        }
    }

    let mut buf = vec![0i16; L1_SIZE];
    for i in 0..L1_SIZE {
        let qf = (RAW_NET[i] * QA as f32).trunc();
        let q = qf as i16;
        assert_eq!(f32::from(q), qf);
        buf[i] = q;
    }

    write_buf(&buf, &mut file);
    write_buf(&RAW_NET[L1_SIZE..], &mut file);
}
