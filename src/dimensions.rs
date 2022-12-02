
use std::{ops::Mul, fmt::Display, str::FromStr};

use derive_more::{From, Sub, Div};
use getset::CopyGetters;
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;


#[derive(CopyGetters, PartialEq, Eq, PartialOrd, Ord, From, Debug, Clone, Copy, Div, Sub)]
#[getset(get_copy = "pub")]
pub struct Dimensions<T: PartialEq + Eq + PartialOrd + Ord + Copy + Clone + Display> {
    pub width: T,
    pub height: T
}

impl<T: PartialEq + Eq + PartialOrd + Ord + Copy + Display> Dimensions<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

impl<T: PartialEq + Eq + PartialOrd + Ord + Copy + Display> Display for Dimensions<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

// impl<T: PartialOrd + Ord + Copy + Div<Output = T>> Div<T> for Dimensions<T> {
//     type Output = Self;

//     fn div(self, rhs: T) -> Self::Output {
//         Self { width: self.width / rhs, height: self.height / rhs }
//     }
// }

impl<T: PartialOrd + Ord + Copy + Mul<Output = T> + Display> Mul<T> for Dimensions<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self { width: self.width * rhs, height: self.height * rhs }
    }
}

impl<T: PartialOrd + Ord + Copy + Mul<Output = T> + Display> Mul for Dimensions<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self { width: self.width * rhs.width, height: self.height * rhs.height }
    }
}

#[derive(Debug, Error)]
#[error("invalid dimensions format: {0}")]
pub struct FormatError(String);

impl<T> FromStr for Dimensions<T>
where
    T: PartialEq + Eq + PartialOrd + Ord + Copy + Clone + Display + FromStr,
    <T as FromStr>::Err: std::fmt::Debug
{
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! { static ref ORIGIN_RE: Regex = Regex::new(r"\A(?P<width>\d+)x(?P<height>\d+)\z").unwrap(); }
        match ORIGIN_RE.captures(s) {
            Some(captures) => {
                let width: T = captures.name("width").unwrap().as_str().parse().map_err(|_| FormatError(s.to_owned()))?;
                let height: T = captures.name("height").unwrap().as_str().parse().map_err(|_| FormatError(s.to_owned()))?;
                Ok(Self { width, height })
            },
            None => Err(FormatError(s.to_owned())),
        }
    }
}
