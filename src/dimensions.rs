
use std::ops::Mul;

use derive_more::{From, Sub, Div};
use getset::CopyGetters;

#[derive(CopyGetters, PartialEq, Eq, PartialOrd, Ord, From, Debug, Clone, Copy, Div, Sub)]
#[getset(get_copy = "pub")]
pub struct Dimensions<T: PartialEq + Eq + PartialOrd + Ord + Copy + Clone> {
    pub width: T,
    pub height: T
}

impl<T: PartialEq + Eq + PartialOrd + Ord + Copy> Dimensions<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

// impl<T: PartialOrd + Ord + Copy + Div<Output = T>> Div<T> for Dimensions<T> {
//     type Output = Self;

//     fn div(self, rhs: T) -> Self::Output {
//         Self { width: self.width / rhs, height: self.height / rhs }
//     }
// }

impl<T: PartialOrd + Ord + Copy + Mul<Output = T>> Mul<T> for Dimensions<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self { width: self.width * rhs, height: self.height * rhs }
    }
}

impl<T: PartialOrd + Ord + Copy + Mul<Output = T>> Mul for Dimensions<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self { width: self.width * rhs.width, height: self.height * rhs.height }
    }
}