use approx::{AbsDiffEq, RelativeEq};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use super::common::ValueType;
use yuki_derive::{
    impl_spectrum, AbsDiffEq, Add, AddAssign, AddAssignScalar, AddScalar, Div, DivAssign,
    DivAssignScalar, DivScalar, Index, IndexMut, Mul, MulAssign, MulAssignScalar, MulScalar, Neg,
    RelativeEq, Sub, SubAssign, SubAssignScalar, SubScalar,
};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Color_and_Radiometry/Spectral_Representation

/// A spectral power distribution stored as RGB
#[impl_spectrum]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    AbsDiffEq,
    RelativeEq,
    Index,
    IndexMut,
    Neg,
    Add,
    Sub,
    Mul,
    Div,
    AddScalar,
    SubScalar,
    MulScalar,
    DivScalar,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    AddAssignScalar,
    SubAssignScalar,
    MulAssignScalar,
    DivAssignScalar,
)]
pub struct Spectrum<T>
where
    T: ValueType,
{
    /// The r component of the spd
    pub r: T,
    /// The g component of the spd
    pub g: T,
    /// The b component of the spd
    pub b: T,
}
