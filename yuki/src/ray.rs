use approx::{AbsDiffEq, RelativeEq};

use crate::common::FloatValueType;
use crate::point::Point3;
use crate::vector::Vec3;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Rays.html

#[derive(Copy, PartialEq, Clone, Debug)]
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

// impl<T> AbsDiffEq for Ray<T>
// where
//     T: FloatValueType + AbsDiffEq + approx::AbsDiffEq<Epsilon = T>,
// {
//     type Epsilon = T::Epsilon;

//     fn default_epsilon() -> Self::Epsilon {
//         T::default_epsilon()
//     }

//     fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
//         self.o.abs_diff_eq(&other.o, epsilon)
//             && self.d.abs_diff_eq(&other.d, epsilon)
//             && self.t_max.abs_diff_eq(&other.t_max, epsilon)
//     }
// }

// impl<T> RelativeEq for Ray<T>
// where
//     T: FloatValueType + RelativeEq + approx::AbsDiffEq<Epsilon = T>,
// {
//     fn default_max_relative() -> Self::Epsilon {
//         T::default_max_relative()
//     }

//     fn relative_eq(
//         &self,
//         other: &Self,
//         epsilon: Self::Epsilon,
//         max_relative: Self::Epsilon,
//     ) -> bool {
//         self.o.relative_eq(&other.o, epsilon)
//             && self.d.relative_eq(&other.d, epsilon)
//             && self.t_max.relative_eq(&other.t_max, epsilon)
//     }
// }
