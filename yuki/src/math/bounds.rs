use num::Integer;
use std::ops::{Index, IndexMut};

use yuki_derive::{impl_bounds, Index, IndexMut};

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
    pub fn area(&self) -> T {
        let d = self.diagonal();
        d.x * d.y
    }

    /// Finds the maximum extent of this `Bounds2`
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
    pub fn surface_area(&self) -> T {
        let d = self.diagonal();
        // A bit dirty but a Num with FromPrimitive should be fine with this cast
        T::from_u8(2).unwrap() * (d.x * d.y + d.z * d.y + d.x * d.z)
    }

    /// Calculates the volume of this `Bounds3`
    pub fn volume(&self) -> T {
        let d = self.diagonal();
        d.x * d.y * d.z
    }

    /// Finds the maximum extent of this `Bounds3`
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

    /// Returns the center and radius of this `Bounds3`'s bounding sphere.
    ///
    /// Returns [None] if there is no valid bounding sphere.
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
    fn slab_test(&self, ray: Ray<T>, inv_dir: Vec3<T>) -> (T, T) {
        // Adapted from slab test used in OptiX and Embree as listed in
        // A Ray-Box Intersection Algorithm and Efficient Dynamic Voxel Rendering
        // by Alexander Majercik et. al.

        // TODO: Implement component-wise multiply for vec3 type
        fn mul_vec3<V: FloatValueType>(v0: Vec3<V>, v1: Vec3<V>) -> Vec3<V> {
            Vec3::new(v0.x * v1.x, v0.y * v1.y, v0.z * v1.z)
        }

        let t0 = mul_vec3(self.p_min - ray.o, inv_dir);
        let t1 = mul_vec3(self.p_max - ray.o, inv_dir);

        // TODO: Ray tmin
        (
            t0.min(t1).max_comp().max(T::zero()),
            t0.max(t1).min_comp().min(ray.t_max),
        )
    }

    /// Returns both intersections of `ray` with this `Bounds3`, if valid.
    pub fn intersections(&self, ray: Ray<T>) -> Option<(T, T)> {
        let inv_dir = Vec3::new(T::one() / ray.d.x, T::one() / ray.d.y, T::one() / ray.d.z);
        let (tmin, tmax) = self.slab_test(ray, inv_dir);

        if tmin <= tmax {
            Some((tmin, tmax))
        } else {
            None
        }
    }

    /// Checks if `ray` hits this `Bounds3`.
    ///
    /// Precomputed `inv_dir` is supplied as an optimization.
    pub fn intersect(&self, ray: Ray<T>, inv_dir: Vec3<T>) -> bool {
        let (tmin, tmax) = self.slab_test(ray, inv_dir);

        tmin <= tmax
    }
}
