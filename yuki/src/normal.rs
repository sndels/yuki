use approx::{AbsDiffEq, RelativeEq};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use crate::common::FloatValueType;
use crate::vector::Vec3;
use yuki_derive::*;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Normals.html

/// A three-dimensional surface normal
///
/// Note that a [Normal] is not necessarily normalized as it is merely a vector perpendicular
/// to a surface at a position on it.
#[impl_normal]
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
    MulScalar,
    DivScalar,
    AddAssign,
    SubAssign,
    MulAssignScalar,
    DivAssignScalar,
)]
pub struct Normal<T>
where
    T: FloatValueType,
{
    /// The x component of the normal
    pub x: T,
    /// The y component of the normal
    pub y: T,
    /// The z component of the normal
    pub z: T,
}

impl<T> Normal<T>
where
    T: FloatValueType,
{
    #[inline]
    /// Calculates the dot product of this `Normal` and a [Vec3].
    pub fn dot_v(&self, v: Vec3<T>) -> T {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
}

impl<T> From<Vec3<T>> for Normal<T>
where
    T: FloatValueType,
{
    fn from(v: Vec3<T>) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}
