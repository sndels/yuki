use crate::common::FloatValueType;
use crate::point::Point3;
use crate::vector::Vec3;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Rays.html

#[derive(Copy, Clone, Debug)]
pub struct Ray<T>
where
    T: FloatValueType,
{
    pub o: Point3<T>,
    pub d: Vec3<T>,
    pub t_max: T,
    // TODO: Time
    // TODO: Medium
}

impl<T> Ray<T>
where
    T: FloatValueType,
{
    /// Creates a new `Ray`.
    pub fn new(o: Point3<T>, d: Vec3<T>, t_max: T) -> Self {
        let ret = Self { o, d, t_max };
        debug_assert!(!ret.has_nans());
        ret
    }

    /// Creates a new infinite `Ray` from origin toward positive Y.
    pub fn default() -> Self {
        Self {
            o: Point3::zeros(),
            d: Vec3::new(T::zero(), T::one(), T::zero()),
            t_max: T::infinity(),
        }
    }

    /// Checks if any of the members in this `Ray` contain NaNs.
    pub fn has_nans(&self) -> bool {
        self.o.has_nans() || self.d.has_nans() || self.t_max.is_nan()
    }

    /// Finds the [Point3] on this `Ray` at distance `t`.
    pub fn point(&self, t: T) -> Point3<T> {
        self.o + self.d * t
    }
}
