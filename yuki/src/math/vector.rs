use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use yuki_derive::{
    impl_vec, AbsDiffEq, Add, AddAssign, AddAssignScalar, AddScalar, DivAssignScalar, DivScalar,
    Index, IndexMut, MulAssignScalar, MulScalar, Neg, RelativeEq, Sub, SubAssign, SubAssignScalar,
    SubScalar,
};

use super::{
    common::{FloatValueType, ValueType},
    normal::Normal,
    point::Point3,
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html

/// A two-dimensional vector.
#[allow(clippy::unsafe_derive_deserialize)] // This is a POD
#[impl_vec]
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    AbsDiffEq,
    RelativeEq,
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
    /// The x component of the vector.
    pub x: T,
    /// The y component of the vector.
    pub y: T,
}

/// A three-dimensional vector.
#[impl_vec]
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    AbsDiffEq,
    RelativeEq,
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
    /// The x component of the vector.
    pub x: T,
    /// The y component of the vector.
    pub y: T,
    /// The z component of the vector.
    pub z: T,
}

/// A four-dimensional vector.
#[impl_vec]
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    AbsDiffEq,
    RelativeEq,
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
    /// The x component of the vector.
    pub x: T,
    /// The y component of the vector.
    pub y: T,
    /// The z component of the vector.
    pub z: T,
    /// The w component of the vector.
    pub w: T,
}

impl<T> Vec2<T>
where
    T: ValueType,
{
    /// Finds the value of the minimum componen in this `Vec3`t.
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.mini(self.y)
    }

    /// Finds the value of the maximum component in this `Vec3`.
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.maxi(self.y)
    }

    /// Finds the index of the maximum component in this `Vec3`.
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
    /// Finds the value of the minimum component in this `Vec3`.
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.mini(self.y.mini(self.z))
    }

    /// Finds the value of the maximum component in this `Vec3`.
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.maxi(self.y.maxi(self.z))
    }

    /// Finds the index of the maximum component in this `Vec3`.
    pub fn max_dimension(&self) -> usize {
        debug_assert!(!self.has_nans());

        if self.x > self.y {
            if self.x > self.z {
                0
            } else {
                2
            }
        } else if self.y > self.z {
            1
        } else {
            2
        }
    }
}

impl<T> From<Normal<T>> for Vec3<T>
where
    T: FloatValueType,
{
    fn from(n: Normal<T>) -> Self {
        Self::new(n.x, n.y, n.z)
    }
}

impl<T> From<Point3<T>> for Vec3<T>
where
    T: FloatValueType,
{
    fn from(p: Point3<T>) -> Self {
        Self::new(p.x, p.y, p.z)
    }
}

impl<T> Vec3<T>
where
    T: FloatValueType,
{
    /// Calculates the dot product of this `Vec3` and a [Normal].
    pub fn dot_n(&self, n: Normal<T>) -> T {
        self.x * n.x + self.y * n.y + self.z * n.z
    }

    /// Calculates the cross product of this `Vec3` and another `Vec3`.
    //
    // Always uses `f64` internally to avoid errors on "catastrophic cancellation".
    // http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html#DotandCrossProduct
    pub fn cross(&self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        let v1 = Vec3::new(
            self.x.to_f64().unwrap_or(f64::NAN),
            self.y.to_f64().unwrap_or(f64::NAN),
            self.z.to_f64().unwrap_or(f64::NAN),
        );
        let v2 = Vec3::new(
            other.x.to_f64().unwrap_or(f64::NAN),
            other.y.to_f64().unwrap_or(f64::NAN),
            other.z.to_f64().unwrap_or(f64::NAN),
        );
        Self {
            x: T::from((v1.y * v2.z) - (v1.z * v2.y)).unwrap(),
            y: T::from((v1.z * v2.x) - (v1.x * v2.z)).unwrap(),
            z: T::from((v1.x * v2.y) - (v1.y * v2.x)).unwrap(),
        }
    }
}
