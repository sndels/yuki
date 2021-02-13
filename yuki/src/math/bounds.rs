use num::Integer;
use std::{
    iter::{IntoIterator, Iterator},
    ops::{Index, IndexMut},
};

use yuki_derive::*;

use super::{
    common::{FloatValueType, ValueType},
    point::{Point2, Point3},
    ray::Ray,
    vector::{Vec2, Vec3},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Bounding_Boxes.html

/// Two-dimensional bounds.
#[impl_bounds]
#[derive(Copy, Clone, Debug, PartialEq, Index, IndexMut)]
pub struct Bounds2<T>
where
    T: ValueType,
{
    /// The minimum extent of the bounds.
    pub p_min: Point2<T>,
    /// The maximum extent of the bounds.
    pub p_max: Point2<T>,
}

/// Three-dimensional bounds.
#[impl_bounds]
#[derive(Copy, Clone, Debug, PartialEq, Index, IndexMut)]
pub struct Bounds3<T>
where
    T: ValueType,
{
    /// The minimum extent of the bounds.
    pub p_min: Point3<T>,
    /// The maximum extent of the bounds.
    pub p_max: Point3<T>,
}

impl<T> Bounds2<T>
where
    T: ValueType,
{
    /// Calculates the area of this `Bounds2`
    #[inline]
    pub fn area(&self) -> T {
        let d = self.diagonal();
        d.x * d.y
    }

    /// Finds the maximum extent of this `Bounds2`
    #[inline]
    pub fn maximum_extent(&self) -> usize {
        let d = self.diagonal();
        if d.x > d.y {
            0
        } else {
            1
        }
    }
}

/// A row-by-row iterator over the [Point2]s in a `Bounds2`.
/// Starts from `p_min` and excludes the upper bounds.
pub struct Bounds2IntoIter<T>
where
    T: ValueType + Integer,
{
    bb: Bounds2<T>,
    curr: Point2<T>,
}

/// A row-by-row iterator over the [Point2]s in a `Bounds2`.
/// Starts from `p_min` and excludes the upper bounds.
impl<T> IntoIterator for Bounds2<T>
where
    T: ValueType + Integer,
{
    type Item = Point2<T>;
    type IntoIter = Bounds2IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        assert!(
            self.p_min.x < self.p_max.x && self.p_min.y < self.p_max.y,
            "Bounds2 with a dimension <= 0"
        );
        Bounds2IntoIter {
            bb: self,
            curr: self.p_min,
        }
    }
}

impl<T> Iterator for Bounds2IntoIter<T>
where
    T: ValueType + Integer,
{
    type Item = Point2<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // We exclude the max bound
        let ret = if self.curr.y >= self.bb.p_max.y {
            None
        } else {
            Some(self.curr)
        };

        if ret.is_some() {
            self.curr.x += T::one();
            // We exclude the max bound
            if self.curr.x >= self.bb.p_max.x {
                self.curr.x = self.bb.p_min.x;
                self.curr.y += T::one();
            }
        }

        ret
    }
}

impl<T> Bounds3<T>
where
    T: ValueType,
{
    /// Calculates the surface area of this `Bounds3`
    #[inline]
    pub fn surface_area(&self) -> T {
        let d = self.diagonal();
        // A bit dirty but a Num with FromPrimitive should be fine with this cast
        T::from_u8(2).unwrap() * (d.x * d.y + d.z * d.y + d.x * d.z)
    }

    /// Calculates the volume of this `Bounds3`
    #[inline]
    pub fn volume(&self) -> T {
        let d = self.diagonal();
        d.x * d.y * d.z
    }

    /// Finds the maximum extent of this `Bounds3`
    #[inline]
    pub fn maximum_extent(&self) -> usize {
        let d = self.diagonal();
        if d.x > d.y && d.x > d.z {
            0
        } else if d.y > d.z {
            1
        } else {
            2
        }
    }

    /// Returns the center and radius of this `Bounds3`'s bounding sphere. Returns `None` if there is no valid bounding sphere.
    #[inline]
    pub fn bounding_sphere(&self) -> Option<(Point3<T>, T)> {
        // The unwrap is a bit dirty but a Num with FromPrimitive should be fine with this cast
        let center = (self.p_min + self.p_max) / T::from_u32(2).unwrap();
        if self.inside(center) {
            Some((center, self.p_max.dist(center)))
        } else {
            None
        }
    }
}

impl<T> Bounds3<T>
where
    T: FloatValueType,
{
    /// Checks if `ray` hits this `Bounds3`
    /// `inv_dir` and `dir_is_neg` precomputed from `ray` are supplied an optimization
    pub fn intersect(&self, ray: Ray<T>, inv_dir: Vec3<T>, dir_is_neg: [bool; 3]) -> bool {
        // X-slabs test
        let mut t0 = (self[dir_is_neg[0] as usize].x - ray.o.x) * inv_dir.x;
        let mut t1 = (self[1 - (dir_is_neg[0] as usize)].x - ray.o.x) * inv_dir.x;

        // Y,Z -slabs test
        for i in 1..3 {
            let ti0 = (self[dir_is_neg[i] as usize][i] - ray.o[i]) * inv_dir[i];
            let ti1 = (self[1 - (dir_is_neg[i] as usize)][i] - ray.o[i]) * inv_dir[i];
            if t0 > ti1 || ti0 > t1 {
                return false;
            }
            t0 = t0.min(ti0);
            t1 = t1.min(ti1);
        }

        return t0 < ray.t_max && t1 > T::zero();
    }
}
