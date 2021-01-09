use num::cast::{FromPrimitive, ToPrimitive};
use num::traits::{Float, Num};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// Generic types that can be stored in the lib containers
pub trait ValueType:
    Num
    + Mini
    + Maxi
    + PartialOrd
    + ToPrimitive
    + FromPrimitive
    + Copy
    + Add
    + AddAssign
    + Div
    + DivAssign
    + Mul
    + MulAssign
    + Sub
    + SubAssign
{
}
pub trait FloatValueType: ValueType + Float {}

// Impls for all matching types
impl<T> ValueType for T where
    T: Num
        + Mini
        + Maxi
        + PartialOrd
        + ToPrimitive
        + FromPrimitive
        + Copy
        + Add
        + AddAssign
        + Div
        + DivAssign
        + Mul
        + MulAssign
        + Sub
        + SubAssign
{
}
impl<T> FloatValueType for T where T: ValueType + Float {}

/// Trait that maps to number types that implement `fn min(&self, other)`
pub trait Mini {
    /// Returns self.max(other)
    fn mini(&self, other: Self) -> Self;
}

/// Trait that maps to number types that implement `fn max(&self, other)`
pub trait Maxi {
    /// Returns self.max(other)
    fn maxi(&self, other: Self) -> Self;
}

macro_rules! impl_mini_float {
    ( $( $t:ty ),+ ) => {
        $(
            impl Mini for $t {
                fn mini(&self, other: $t) -> $t {
                    self.min(other)
                }
            }
        )*
    }
}
impl_mini_float!(f32, f64);

macro_rules! impl_mini_float {
    ( $( $t:ty ),+ ) => {
        $(
            impl Maxi for $t {
                fn maxi(&self, other: $t) -> $t {
                    self.max(other)
                }
            }
        )*
    }
}
impl_mini_float!(f32, f64);

macro_rules! impl_mini_integer {
    ( $( $t:ty ),+ ) => {
        $(
            impl Mini for $t {
                fn mini(&self, other: $t) -> $t {
                    *self.min(&other)
                }
            }
        )*
    }
}
impl_mini_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

macro_rules! impl_maxi_integer {
    ( $( $t:ty ),+ ) => {
        $(
            impl Maxi for $t {
                fn maxi(&self, other: $t) -> $t {
                    *self.max(&other)
                }
            }
        )*
    }
}
impl_maxi_integer!(u8, u16, u32, u64, i8, i16, i32, i64);
