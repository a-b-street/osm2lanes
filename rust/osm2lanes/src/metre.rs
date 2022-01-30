use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Metre(f64);

impl Metre {
    pub const fn new(val: f64) -> Self {
        Self(val)
    }
    pub const fn val(&self) -> f64 {
        self.0
    }
}

impl std::ops::Add for Metre {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}
impl std::ops::AddAssign for Metre {
    fn add_assign(&mut self, other: Self) {
        *self = Self(self.0 + other.0);
    }
}
impl std::ops::Mul<Metre> for f64 {
    // The division of rational numbers is a closed operation.
    type Output = Metre;
    fn mul(self, other: Metre) -> Self::Output {
        Metre::new(self * other.val())
    }
}
impl std::iter::Sum for Metre {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Metre>,
    {
        Self(iter.map(|m| m.0).sum())
    }
}
