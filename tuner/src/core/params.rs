use super::{NUM_PARAMS, S};
use std::ops::*;

pub struct Params(Box<[S; NUM_PARAMS]>);

impl Default for Params {
    fn default() -> Self {
        Params(Box::new([S::new(0.); NUM_PARAMS]))
    }
}

impl Params {
    pub fn write_to_bin(&self, output_path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(output_path)?;
        let mut params = [(0i32, 0i32); NUM_PARAMS];
        for (i, param) in self.0.iter().enumerate() {
            params[i] = (param.0 as i32, param.1 as i32);
        }
        unsafe {
            file.write_all(
                &std::mem::transmute::<[(i32, i32); NUM_PARAMS], [u8; NUM_PARAMS * 8]>(params)
            )?;
        }
        Ok(())
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
