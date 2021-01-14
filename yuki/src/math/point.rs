use approx::{AbsDiffEq, RelativeEq};
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};

use super::common::ValueType;
use yuki_derive::*;

use super::vector::{Vec2, Vec3};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Points.html

// Note about Point ops:
// Some don't really make mathematical sense but are useful in weighted sums
// point + point = point
// point * scalar = point
// point *= scalar

/// A two-dimensional point.
#[impl_point]
#[vec_op(Add Vec2 Point2)]
#[vec_op(Add Point2 Point2)]
#[vec_op(Sub Vec2 Point2)]
#[vec_op(Sub Point2 Vec2)]
#[vec_assign_op(AddAssign Vec2)]
#[vec_assign_op(SubAssign Vec2)]
#[derive(
    Copy,
    Clone,
    Debug,
    AbsDiffEq,
    RelativeEq,
    PartialEq,
    Index,
    IndexMut,
    AddAssign,
    DivScalar,
    MulScalar,
    DivAssignScalar,
    MulAssignScalar,
)]
pub struct Point2<T>
where
    T: ValueType,
{
    /// The x component of the point.
    pub x: T,
    /// The y component of the point.
    pub y: T,
}

/// A three-dimensional point.
#[impl_point]
#[vec_op(Add Vec3 Point3)]
#[vec_op(Add Point3 Point3)]
#[vec_op(Sub Vec3 Point3)]
#[vec_op(Sub Point3 Vec3)]
#[vec_assign_op(AddAssign Vec3)]
#[vec_assign_op(SubAssign Vec3)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    AbsDiffEq,
    RelativeEq,
    Index,
    IndexMut,
    AddAssign,
    DivScalar,
    MulScalar,
    DivAssignScalar,
    MulAssignScalar,
)]
pub struct Point3<T>
where
    T: ValueType,
{
    /// The x component of the point.
    pub x: T,
    /// The y component of the point.
    pub y: T,
    /// The z component of the point.
    pub z: T,
}
