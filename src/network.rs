const L1: usize = 768;
const L2: usize = 16;
const SCALE: i32 = 400;
const QA: i32 = 181;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[repr(C)]
pub struct Network {
    l1_w: [Accumulator; 768],
    l1_b: Accumulator,
    l2_w: [[[i16; L2]; L1]; 2],
    l2_b: [i16; L2],
    l3: Layer<L2, 1>,
}

static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../resources/net-05.01.24-epoch17.bin")) };

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator) -> i32 {
        let mut l2 = [0; L2];
        flatten(boys, &NNUE.l2_w[0], &mut l2);
        flatten(opps, &NNUE.l2_w[1], &mut l2);

        let mut l2 = dequantise(&l2);
        l2.crelu();

        let l3 = NNUE.l3.out(&l2);

        (l3.vals[0] * SCALE as f32) as i32
    }
}

fn flatten(acc: &Accumulator, weights: &[[i16; L2]; L1], out: &mut [i32; L2]) {
    for (&x, &col) in acc.vals.iter().zip(weights.iter()) {
        let act = screlu(x);

        for (i, &j) in out.iter_mut().zip(col.iter()) {
            *i += act * i32::from(j);
        }
    }
}

#[inline]
fn screlu(x: i16) -> i32 {
    i32::from(x.clamp(0, QA as i16)).pow(2)
}

/// 1 if not SCReLU
const ACCUMULATED_Q: i32 = QA;


fn dequantise<const N: usize>(inp: &[i32; N]) -> Column<N> {
    let mut out = Column::default();

    for (i, &j) in out.vals.iter_mut().zip(inp.iter()) {
        *i = (j / ACCUMULATED_Q) as f32 / QAB as f32;
    }

    out
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Layer<const M: usize, const N: usize> {
    weights: [Column<N>; M],
    bias: Column<N>,
}

impl<const M: usize, const N: usize> Layer<M, N> {
    fn out(&self, inp: &Column<M>) -> Column<N> {
        let mut out = self.bias;

        for (&mul, col) in inp.vals.iter().zip(self.weights.iter()) {
            out.madd_assign(mul, col);
        }

        out
    }
}

#[derive(Clone, Copy)]
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
    fn madd_assign(&mut self, mul: f32, rhs: &Column<N>) {
        for (i, &j) in self
            .vals
            .iter_mut()
            .zip(rhs.vals.iter())
        {
            *i += mul * j;
        }
    }

    fn crelu(&mut self) {
        for val in self.vals.iter_mut() {
            *val = val.clamp(0.0, 1.0);
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
    const L3_SIZE: usize = L2 + 1;
    const SIZE: usize = L1_SIZE + L2_SIZE + L3_SIZE;

    static RAW_NET: [f32; SIZE] = unsafe {
        std::mem::transmute(*include_bytes!("../../bullet/checkpoints/net-05.01.24-epoch17/params.bin"))
    };

    let mut file = File::create("resources/net-05.01.24-epoch17.bin").unwrap();

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

    let mut buf = vec![0i16; L2_SIZE];
    for i in 0..L2_SIZE {
        let qf = (RAW_NET[L1_SIZE + i] * QB as f32).trunc();
        let q = qf as i16;
        assert_eq!(f32::from(q), qf);
        buf[i] = q;
    }

    write_buf(&buf, &mut file);

    write_buf(&RAW_NET[L1_SIZE + L2_SIZE..SIZE], &mut file);
}
