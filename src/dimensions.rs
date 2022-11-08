
use derive_more::From;
use getset::CopyGetters;

#[derive(CopyGetters, PartialEq, Eq, PartialOrd, Ord, From, Debug)]
#[getset(get_copy = "pub")]
pub struct Dimensions<T: PartialEq + Eq + PartialOrd + Ord + Copy> {
    pub(crate) width: T,
    pub(crate) height: T
}

impl<T: PartialEq + Eq + PartialOrd + Ord + Copy> Dimensions<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}