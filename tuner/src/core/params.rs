use super::{NUM_PARAMS, S};
use std::ops::*;

pub struct Params([S; NUM_PARAMS]);

impl Default for Params {
    fn default() -> Self {
        Params::new([S::new(0.); NUM_PARAMS])
    }
}

impl Params {
    /// Constructs new Params with given values.
    pub const fn new(vals: [S; NUM_PARAMS]) -> Self {
        Self(vals)
    }

    /// Outputs a table of parameters with given shape, starting from given index.
    pub fn output_table(&self, start: usize, rows: usize, columns: usize) {
        println!("[");
        for i in 0..rows {
            let s: String = self.0[start + columns * i..start + columns * (i + 1)]
                .iter()
                .map(|s| format!(" {},", s.fancy()))
                .collect();
            println!("   {s}")
        }
        println!("],");
    }
}

impl Index<u16> for Params {
    type Output = S;
    fn index(&self, index: u16) -> &Self::Output {
        &self.0[usize::from(index)]
    }
}

impl IndexMut<u16> for Params {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        &mut self.0[usize::from(index)]
    }
}

impl Add<Params> for Params {
    type Output = Params;
    fn add(mut self, rhs: Params) -> Self::Output {
        for i in 0..NUM_PARAMS {
            self.0[i] += rhs.0[i];
        }
        self
    }
}

impl Sub<Params> for Params {
    type Output = Params;
    fn sub(mut self, rhs: Params) -> Self::Output {
        for i in 0..NUM_PARAMS {
            self.0[i] -= rhs.0[i];
        }
        self
    }
}

impl AddAssign<Params> for Params {
    fn add_assign(&mut self, rhs: Params) {
        for i in 0..NUM_PARAMS {
            self.0[i] += rhs.0[i]
        }
    }
}
