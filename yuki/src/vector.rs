use num::traits::Float;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use crate::common::ValueType;
use yuki_derive::*;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html

/// A two-dimensional vector
#[impl_vec]
#[impl_abs_diff_eq(f32, f64)]
#[impl_relative_eq(f32, f64)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Index,
    IndexMut,
    Neg,
    Add,
    Sub,
    AddScalar,
    SubScalar,
    MulScalar,
    DivScalar,
    AddAssign,
    SubAssign,
    AddAssignScalar,
    SubAssignScalar,
    MulAssignScalar,
    DivAssignScalar,
)]
pub struct Vec2<T>
where
    T: ValueType,
{
    /// The x component of the vector
    pub x: T,
    /// The y component of the vector
    pub y: T,
}

/// A three-dimensional vector
#[impl_vec]
#[impl_abs_diff_eq(f32, f64)]
#[impl_relative_eq(f32, f64)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Index,
    IndexMut,
    Neg,
    Add,
    Sub,
    AddScalar,
    SubScalar,
    MulScalar,
    DivScalar,
    AddAssign,
    SubAssign,
    AddAssignScalar,
    SubAssignScalar,
    MulAssignScalar,
    DivAssignScalar,
)]
pub struct Vec3<T>
where
    T: ValueType,
{
    /// The x component of the vector
    pub x: T,
    /// The y component of the vector
    pub y: T,
    /// The z component of the vector
    pub z: T,
}

/// A four-dimensional vector
#[impl_vec]
#[impl_abs_diff_eq(f32, f64)]
#[impl_relative_eq(f32, f64)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Index,
    IndexMut,
    Neg,
    Add,
    Sub,
    AddScalar,
    SubScalar,
    MulScalar,
    DivScalar,
    AddAssign,
    SubAssign,
    AddAssignScalar,
    SubAssignScalar,
    MulAssignScalar,
    DivAssignScalar,
)]
pub struct Vec4<T>
where
    T: ValueType,
{
    /// The x component of the vector
    pub x: T,
    /// The y component of the vector
    pub y: T,
    /// The z component of the vector
    pub z: T,
    /// The w component of the vector
    pub w: T,
}

impl<T> Vec2<T>
where
    T: ValueType,
{
    /// Returns the value of the minumum component.
    #[inline]
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.mini(self.y)
    }

    /// Returns the value of the maximum component.
    #[inline]
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.maxi(self.y)
    }

    /// Returns the index of the maximum component.
    #[inline]
    pub fn max_dimension(&self) -> usize {
        debug_assert!(!self.has_nans());

        if self.x > self.y {
            0
        } else {
            1
        }
    }
}

impl<T> Vec3<T>
where
    T: ValueType,
{
    /// Returns the value of the minumum component.
    #[inline]
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.mini(self.y.mini(self.z))
    }

    /// Returns the value of the maximum component.
    #[inline]
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.maxi(self.y.maxi(self.z))
    }

    /// Returns the index of the maximum component.
    #[inline]
    pub fn max_dimension(&self) -> usize {
        debug_assert!(!self.has_nans());

        if self.x > self.y {
            if self.x > self.z {
                0
            } else {
                2
            }
        } else {
            if self.y > self.z {
                1
            } else {
                2
            }
        }
    }
}

impl<T> Vec3<T>
where
    T: Float + ValueType,
{
    /// Returns the cross product of the two vectors.
    //
    // Always uses `f64` internally to avoid errors on "catastrophic cancellation".
    // See pbrt [2.2.1](http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html#DotandCrossProduct) for details
    #[inline]
    pub fn cross(&self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        let v1x = self.x.to_f64().unwrap_or(f64::NAN);
        let v1y = self.y.to_f64().unwrap_or(f64::NAN);
        let v1z = self.z.to_f64().unwrap_or(f64::NAN);
        let v2x = other.x.to_f64().unwrap_or(f64::NAN);
        let v2y = other.y.to_f64().unwrap_or(f64::NAN);
        let v2z = other.z.to_f64().unwrap_or(f64::NAN);
        Self {
            x: T::from((v1y * v2z) - (v1z * v2y)).unwrap(),
            y: T::from((v1z * v2x) - (v1x * v2z)).unwrap(),
            z: T::from((v1x * v2y) - (v1y * v2y)).unwrap(),
        }
    }
}

impl<T> Vec4<T>
where
    T: ValueType,
{
    /// Returns the value of the minumum component.
    #[inline]
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        let a = self.x.mini(self.y);
        let b = self.z.mini(self.w);
        a.mini(b)
    }

    /// Returns the value of the maximum component.
    #[inline]
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        let a = self.x.maxi(self.y);
        let b = self.z.maxi(self.w);
        a.maxi(b)
    }

    /// Returns the index of the maximum component.
    #[inline]
    pub fn max_dimension(&self) -> usize {
        debug_assert!(!self.has_nans());

        if self.x > self.y {
            if self.x > self.z {
                if self.x > self.w {
                    0
                } else {
                    3
                }
            } else {
                if self.z > self.w {
                    2
                } else {
                    3
                }
            }
        } else {
            if self.y > self.z {
                if self.y > self.w {
                    1
                } else {
                    3
                }
            } else {
                if self.z > self.w {
                    2
                } else {
                    3
                }
            }
        }
    }
}
