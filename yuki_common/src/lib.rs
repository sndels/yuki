use num::cast::{FromPrimitive, ToPrimitive};
use num::traits::Num;

/// Generic type that can be stored in the lib containers
pub trait ValueType: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy {}

// Impl for all matching types
impl<T> ValueType for T where T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy {}

/// Helper trait to generalize on types that implement `fn min(self,other)`
pub trait Mini {
    fn mini(&self, other: Self) -> Self;
}

/// Helper trait to generalize on types that implement `fn max(self, other)`
pub trait Maxi {
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
