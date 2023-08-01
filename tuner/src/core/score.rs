use std::{ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Sub, SubAssign}, fmt::Debug};

/// S with a midgame and endgame value.
#[derive(Clone, Copy, Default)]
pub struct S(pub f64, pub f64);

impl Add<S> for S {
    type Output = S;
    fn add(self, rhs: S) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Add<f64> for S {
    type Output = S;
    fn add(self, rhs: f64) -> Self::Output {
        Self(self.0 + rhs, self.1 + rhs)
    }
}

impl Sub<S> for S {
    type Output = S;
    fn sub(self, rhs: S) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl AddAssign<f64> for S {
    fn add_assign(&mut self, rhs: f64) {
        self.0 += rhs;
        self.1 += rhs;
    }
}

impl SubAssign<S> for S {
    fn sub_assign(&mut self, rhs: S) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl SubAssign<f64> for S {
    fn sub_assign(&mut self, rhs: f64) {
        self.0 -= rhs;
        self.1 -= rhs;
    }
}

impl Mul<S> for f64 {
    type Output = S;
    fn mul(self, rhs: S) -> Self::Output {
        S(self * rhs.0, self * rhs.1)
    }
}

impl Mul<S> for S {
    type Output = S;
    fn mul(self, rhs: S) -> Self::Output {
        S(self.0 * rhs.0, self.1 * rhs.1)
    }
}

impl Div<f64> for S {
    type Output = S;
    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs, self.1 / rhs)
    }
}

impl Div<S> for S {
    type Output = S;
    fn div(self, rhs: S) -> Self::Output {
        Self(self.0 / rhs.0, self.1 / rhs.1)
    }
}

impl Index<bool> for S {
    type Output = f64;
    fn index(&self, index: bool) -> &Self::Output {
        if index {
            &self.1
        } else {
            &self.0
        }
    }
}

impl IndexMut<bool> for S {
    fn index_mut(&mut self, index: bool) -> &mut Self::Output {
        if index {
            &mut self.1
        } else {
            &mut self.0
        }
    }
}

impl S {
    /// Creates a new instance with both fields set to the argument.
    #[inline]
    pub const fn new(s: f64) -> Self {
        Self(s, s)
    }

    /// Fancy string formatting.
    pub fn fancy(&self) -> String {
        format!("S({: >3.0},{: >4.0})", self.0, self.1)
    }

    pub fn sqrt(&self) -> Self {
        Self(self.0.sqrt(), self.1.sqrt())
    }
}

impl Debug for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S({:.0}, {:.0})", self.0, self.1)
    }
}
