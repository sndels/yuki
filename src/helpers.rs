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

impl Mini for f32 {
    fn mini(&self, other: f32) -> f32 {
        self.min(other)
    }
}
impl Mini for f64 {
    fn mini(&self, other: f64) -> f64 {
        self.min(other)
    }
}
impl Maxi for f32 {
    fn maxi(&self, other: f32) -> f32 {
        self.max(other)
    }
}
impl Maxi for f64 {
    fn maxi(&self, other: f64) -> f64 {
        self.max(other)
    }
}

macro_rules! mini_impl_integer {
    ($t:ty) => {
        impl Mini for $t {
            fn mini(&self, other: $t) -> $t {
                *self.min(&other)
            }
        }
    };
}

mini_impl_integer!(u8);
mini_impl_integer!(u16);
mini_impl_integer!(u32);
mini_impl_integer!(u64);
mini_impl_integer!(i8);
mini_impl_integer!(i16);
mini_impl_integer!(i32);
mini_impl_integer!(i64);

macro_rules! maxi_impl_integer {
    ($t:ty) => {
        impl Maxi for $t {
            fn maxi(&self, other: $t) -> $t {
                *self.max(&other)
            }
        }
    };
}

maxi_impl_integer!(u8);
maxi_impl_integer!(u16);
maxi_impl_integer!(u32);
maxi_impl_integer!(u64);
maxi_impl_integer!(i8);
maxi_impl_integer!(i16);
maxi_impl_integer!(i32);
maxi_impl_integer!(i64);
