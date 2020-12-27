use num::traits::{Float, Signed};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use crate::common::ValueType;
use yuki_derive::*;

use crate::impl_vec_approx_eq;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html

/// A two-dimensional vector
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Index,
    IndexMut,
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
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Index,
    IndexMut,
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
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Index,
    IndexMut,
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

macro_rules! impl_vec {
    ( $( $vec_type:ident
         [ $( $component:ident )+ ]
         $shorthand:ident
       ),+
    ) => {
        $(
            impl<T> $vec_type<T>
            where
                T: ValueType,
            {
                /// Constructs a new vector.
                ///
                /// Has a debug assert that checks for NaNs.
                #[inline]
                pub fn new($($component: T),*) -> Self {
                    let v = Self{ $($component),* };
                    debug_assert!(!v.has_nans());
                    v
                }

                /// Constructs a new vector of 0s.
                #[inline]
                pub fn zeros() -> Self {
                    Self {
                        $($component: T::zero(),)*
                    }
                }

                /// Constructs a new vector of 1s.
                #[inline]
                pub fn ones() -> Self {
                    Self {
                        $($component: T::one(),)*
                    }
                }

                /// Returns `true` if any component is NaN.
                #[inline]
                pub fn has_nans(&self) -> bool {
                    // Not all T have is_nan()
                    $(self.$component != self.$component)||*
                }

                /// Returns the vector's squared length.
                #[inline]
                pub fn len_sqr(&self) -> T {
                    debug_assert!(!self.has_nans());

                    self.dot(*self)
                }

                /// Returns the vector's length.
                #[inline]
                pub fn len(&self) -> T {
                    debug_assert!(!self.has_nans());

                    T::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
                }

                /// Returns the normalized vector.
                #[inline]
                pub fn normalized(&self) -> Self {
                    debug_assert!(!self.has_nans());

                    *self / self.len()
                }

                /// Returns the component-wise minimum of the two vectors.
                #[inline]
                pub fn min(&self, other: Self) -> Self {
                    debug_assert!(!self.has_nans());
                    debug_assert!(!other.has_nans());

                    Self {
                        $($component: self.$component.mini(other.$component),)*
                    }
                }

                /// Returns the component-wise maximum of the two vectors.
                #[inline]
                pub fn max(&self, other: Self) -> Self {
                    debug_assert!(!self.has_nans());
                    debug_assert!(!other.has_nans());

                    Self {
                        $($component: self.$component.maxi(other.$component),)*
                    }
                }

                /// Returns the vector permutation defined by the indices.
                #[inline]
                pub fn permuted(&self $(, $component: usize)*) -> Self {
                    debug_assert!(!self.has_nans());

                    Self {
                        $($component: self[$component],)*
                    }
                }
            }

            /// Shorthand constructor
            #[inline]
            pub fn $shorthand<T>($($component: T),*) -> $vec_type<T>
            where
                T: ValueType
            {
                // Use new() to catch NANs
                $vec_type::new($($component),*)
            }

            impl<T> From<T> for $vec_type<T>
            where
                T: ValueType,
            {
                fn from(v: T) -> Self {
                    Self {
                        $($component: v,)*
                    }
                }
            }

            impl<T> Neg for $vec_type<T>
            where
                T: Signed + ValueType,
            {
                type Output = Self;

                fn neg(self) -> Self {
                    debug_assert!(!self.has_nans());

                    Self {
                        $($component: -self.$component,)*
                    }
                }
            }
        )*
    };
}
impl_vec!(
    Vec2 [x y] vec2,
    Vec3 [x y z] vec3,
    Vec4 [x y z w] vec4
);

macro_rules! impl_vec_dot {
    // Need to do this separately since we cant separate expansion with '+'
    ($( $vec_type:ident [ $component0:ident $( $component:ident )+ ] ),+ ) => {
        $(
            impl<T> $vec_type<T>
            where
                T: ValueType,
            {
                /// Returns the dot product of the two vectors.
                #[inline]
                pub fn dot(&self, other: Self) -> T {
                    debug_assert!(!self.has_nans());
                    debug_assert!(!other.has_nans());

                    self.$component0 * other.$component0 $(+ self.$component * other.$component)*
                }
            }
       )*
    };
}
impl_vec_dot!(
    Vec2 [x y],
    Vec3 [x y z],
    Vec4 [x y z w]
);

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

impl_vec_approx_eq!(
    Vec2<f32> [x y ],
    Vec3<f32> [x y z],
    Vec4<f32> [x y z w],
    Vec2<f64> [x y ],
    Vec3<f64> [x y z],
    Vec4<f64> [x y z w]
);

#[cfg(test)]
mod tests {
    // These tests are more about catching typos than rigorous verification
    // Thus, some assumptions (read: knowledge) of the implementation are made
    // and the tests should be updated according to potential implementation changes

    use approx::assert_abs_diff_eq;
    use std::panic;

    use crate::vector::{vec2, vec3, vec4, Vec2, Vec3, Vec4};

    #[test]
    fn new() {
        let v = Vec2::new(0.0, 1.0);
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 1.0);

        let v = Vec3::new(0.0, 1.0, 2.0);
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 1.0);
        assert_eq!(v.z, 2.0);

        let v = Vec4::new(0.0, 1.0, 2.0, 3.0);
        assert_eq!(v.x, 0.0f32);
        assert_eq!(v.y, 1.0f32);
        assert_eq!(v.z, 2.0f32);
        assert_eq!(v.w, 3.0f32);
    }

    #[test]
    fn zeros() {
        assert_eq!(Vec2::zeros(), Vec2::new(0, 0));
        assert_eq!(Vec3::zeros(), Vec3::new(0, 0, 0));
        assert_eq!(Vec4::zeros(), Vec4::new(0, 0, 0, 0));
    }

    #[test]
    fn ones() {
        assert_eq!(Vec2::ones(), Vec2::new(1, 1));
        assert_eq!(Vec3::ones(), Vec3::new(1, 1, 1));
        assert_eq!(Vec4::ones(), Vec4::new(1, 1, 1, 1));
    }

    #[test]
    fn from() {
        assert_eq!(Vec2::from(2), Vec2::new(2, 2));
        assert_eq!(Vec3::from(2), Vec3::new(2, 2, 2));
        assert_eq!(Vec4::from(2), Vec4::new(2, 2, 2, 2));
    }

    #[test]
    fn index() {
        let v = Vec2::new(0.0, 1.0);
        assert_eq!(v.x, v[0]);
        assert_eq!(v.y, v[1]);
        assert_eq!(vec2(0.0, 1.0), v);

        let v = Vec3::new(0.0, 1.0, 2.0);
        assert_eq!(v.x, v[0]);
        assert_eq!(v.y, v[1]);
        assert_eq!(v.z, v[2]);
        assert_eq!(vec3(0.0, 1.0, 2.0), v);

        let v = Vec4::new(0.0, 1.0, 2.0, 3.0);
        assert_eq!(v.x, v[0]);
        assert_eq!(v.y, v[1]);
        assert_eq!(v.z, v[2]);
        assert_eq!(v.w, v[3]);
        assert_eq!(vec4(0.0, 1.0, 2.0, 3.0), v);

        let mut v = Vec2::zeros();
        v[0] = 1.0;
        v[1] = 2.0;
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);

        let mut v = Vec3::zeros();
        v[0] = 1.0;
        v[1] = 2.0;
        v[2] = 3.0;
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);

        let mut v = Vec4::zeros();
        v[0] = 1.0;
        v[1] = 2.0;
        v[2] = 3.0;
        v[3] = 4.0;
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);
        assert_eq!(v[3], 4.0);
    }

    #[test]
    fn nan() {
        // Test all permutations with constructor as it should panic
        let result = panic::catch_unwind(|| Vec2::new(f32::NAN, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Vec2::new(0.0, f32::NAN));
        assert!(result.is_err());

        let result = panic::catch_unwind(|| Vec3::new(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Vec3::new(0.0, f32::NAN, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Vec3::new(0.0, 0.0, f32::NAN));
        assert!(result.is_err());

        let result = panic::catch_unwind(|| Vec4::new(f32::NAN, 0.0, 0.0, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Vec4::new(0.0, f32::NAN, 0.0, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Vec4::new(0.0, 0.0, f32::NAN, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Vec4::new(0.0, 0.0, 0.0, f32::NAN));
        assert!(result.is_err());

        // Test shorthand constructors, they call new() internally so only need one test per
        let result = panic::catch_unwind(|| vec2(f32::NAN, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| vec3(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| vec4(f32::NAN, 0.0, 0.0, 0.0));
        assert!(result.is_err());
    }
    #[test]
    fn dot() {
        assert_eq!(Vec2::new(2, 3).len_sqr(), 2 * 2 + 3 * 3);
        assert_eq!(Vec3::new(2, 3, 4).len_sqr(), 2 * 2 + 3 * 3 + 4 * 4);
        assert_eq!(
            Vec4::new(2, 3, 4, 5).len_sqr(),
            2 * 2 + 3 * 3 + 4 * 4 + 5 * 5
        );
    }

    #[test]
    fn len_sqr() {
        assert_eq!(Vec2::new(2, 3).len_sqr(), 2 * 2 + 3 * 3);
        assert_eq!(Vec3::new(2, 3, 4).len_sqr(), 2 * 2 + 3 * 3 + 4 * 4);
        assert_eq!(
            Vec4::new(2, 3, 4, 5).len_sqr(),
            2 * 2 + 3 * 3 + 4 * 4 + 5 * 5
        );
    }

    #[test]
    fn len() {
        assert_abs_diff_eq!(
            Vec2::new(2.0, 3.0).len(),
            (2.0f32 * 2.0f32 + 3.0f32 * 3.0f32).sqrt()
        );
        assert_abs_diff_eq!(
            Vec3::new(2.0, 3.0, 4.0).len(),
            (2.0f32 * 2.0f32 + 3.0f32 * 3.0f32 + 4.0f32 * 4.0f32).sqrt()
        );
        assert_abs_diff_eq!(
            Vec4::new(2.0, 3.0, 4.0, 5.0).len(),
            (2.0f32 * 2.0f32 + 3.0f32 * 3.0f32 + 4.0f32 * 4.0f32 + 5.0f32 * 5.0f32).sqrt()
        );
    }

    #[test]
    fn normalized() {
        assert_abs_diff_eq!(Vec2::new(1.0, 1.0).normalized().len(), 1.0);
        assert_abs_diff_eq!(Vec3::new(1.0, 1.0, 1.0).normalized().len(), 1.0);
        assert_abs_diff_eq!(Vec4::new(1.0, 1.0, 1.0, 1.0).normalized().len(), 1.0);
    }

    #[test]
    fn min() {
        let a = Vec2::new(0, 2);
        let b = Vec2::new(3, 1);
        assert_eq!(a.min(b), Vec2::new(0, 1));
        assert_eq!(a.min(b), b.min(a));

        let a = Vec3::new(0, 2, 4);
        let b = Vec3::new(3, 1, 5);
        assert_eq!(a.min(b), Vec3::new(0, 1, 4));
        assert_eq!(a.min(b), b.min(a));

        let a = Vec4::new(0, 2, 4, 7);
        let b = Vec4::new(3, 1, 5, 6);
        assert_eq!(a.min(b), Vec4::new(0, 1, 4, 6));
        assert_eq!(a.min(b), b.min(a));
    }

    #[test]
    fn max() {
        let a = Vec2::new(0, 2);
        let b = Vec2::new(3, 1);
        assert_eq!(a.max(b), Vec2::new(3, 2));
        assert_eq!(a.max(b), b.max(a));

        let a = Vec3::new(0, 2, 4);
        let b = Vec3::new(3, 1, 5);
        assert_eq!(a.max(b), Vec3::new(3, 2, 5));
        assert_eq!(a.max(b), b.max(a));

        let a = Vec4::new(0, 2, 4, 7);
        let b = Vec4::new(3, 1, 5, 6);
        assert_eq!(a.max(b), Vec4::new(3, 2, 5, 7));
        assert_eq!(a.max(b), b.max(a));
    }

    #[test]
    fn min_comp() {
        assert_eq!(Vec2::new(0.0, 1.0).min_comp(), 0.0);
        assert_eq!(Vec2::new(1.0, 0.0).min_comp(), 0.0);

        assert_eq!(Vec3::new(0.0, 1.0, 2.0).min_comp(), 0.0);
        assert_eq!(Vec3::new(1.0, 0.0, 2.0).min_comp(), 0.0);
        assert_eq!(Vec3::new(2.0, 1.0, 0.0).min_comp(), 0.0);

        assert_eq!(Vec4::new(0.0, 1.0, 2.0, 3.0).min_comp(), 0.0);
        assert_eq!(Vec4::new(0.0, 0.0, 2.0, 3.0).min_comp(), 0.0);
        assert_eq!(Vec4::new(0.0, 1.0, 0.0, 3.0).min_comp(), 0.0);
        assert_eq!(Vec4::new(0.0, 1.0, 2.0, 0.0).min_comp(), 0.0);
    }

    #[test]
    fn max_comp() {
        assert_eq!(Vec2::new(0.0, 1.0).max_comp(), 1.0);
        assert_eq!(Vec2::new(1.0, 0.0).max_comp(), 1.0);

        assert_eq!(Vec3::new(0.0, 1.0, 2.0).max_comp(), 2.0);
        assert_eq!(Vec3::new(0.0, 2.0, 1.0).max_comp(), 2.0);
        assert_eq!(Vec3::new(2.0, 1.0, 0.0).max_comp(), 2.0);

        assert_eq!(Vec4::new(0.0, 1.0, 2.0, 3.0).max_comp(), 3.0);
        assert_eq!(Vec4::new(0.0, 1.0, 3.0, 2.0).max_comp(), 3.0);
        assert_eq!(Vec4::new(0.0, 3.0, 2.0, 1.0).max_comp(), 3.0);
        assert_eq!(Vec4::new(3.0, 1.0, 2.0, 0.0).max_comp(), 3.0);
    }

    #[test]
    fn max_dimension() {
        assert_eq!(Vec2::new(0.0, 1.0).max_dimension(), 1);
        assert_eq!(Vec2::new(1.0, 0.0).max_dimension(), 0);

        assert_eq!(Vec3::new(0.0, 1.0, 2.0).max_dimension(), 2);
        assert_eq!(Vec3::new(0.0, 2.0, 1.0).max_dimension(), 1);
        assert_eq!(Vec3::new(1.0, 0.0, 2.0).max_dimension(), 2);
        assert_eq!(Vec3::new(1.0, 2.0, 0.0).max_dimension(), 1);
        assert_eq!(Vec3::new(2.0, 0.0, 1.0).max_dimension(), 0);
        assert_eq!(Vec3::new(2.0, 1.0, 0.0).max_dimension(), 0);

        assert_eq!(Vec4::new(0.0, 1.0, 2.0, 3.0).max_dimension(), 3);
        assert_eq!(Vec4::new(0.0, 1.0, 3.0, 2.0).max_dimension(), 2);
        assert_eq!(Vec4::new(0.0, 2.0, 1.0, 3.0).max_dimension(), 3);
        assert_eq!(Vec4::new(0.0, 2.0, 3.0, 1.0).max_dimension(), 2);
        assert_eq!(Vec4::new(0.0, 3.0, 1.0, 2.0).max_dimension(), 1);
        assert_eq!(Vec4::new(0.0, 3.0, 2.0, 1.0).max_dimension(), 1);
        assert_eq!(Vec4::new(1.0, 0.0, 2.0, 3.0).max_dimension(), 3);
        assert_eq!(Vec4::new(1.0, 0.0, 3.0, 2.0).max_dimension(), 2);
        assert_eq!(Vec4::new(1.0, 2.0, 0.0, 3.0).max_dimension(), 3);
        assert_eq!(Vec4::new(1.0, 2.0, 3.0, 0.0).max_dimension(), 2);
        assert_eq!(Vec4::new(1.0, 3.0, 0.0, 2.0).max_dimension(), 1);
        assert_eq!(Vec4::new(1.0, 3.0, 2.0, 0.0).max_dimension(), 1);
        assert_eq!(Vec4::new(2.0, 0.0, 1.0, 3.0).max_dimension(), 3);
        assert_eq!(Vec4::new(2.0, 0.0, 3.0, 1.0).max_dimension(), 2);
        assert_eq!(Vec4::new(2.0, 1.0, 0.0, 3.0).max_dimension(), 3);
        assert_eq!(Vec4::new(2.0, 1.0, 3.0, 0.0).max_dimension(), 2);
        assert_eq!(Vec4::new(2.0, 3.0, 0.0, 1.0).max_dimension(), 1);
        assert_eq!(Vec4::new(2.0, 3.0, 1.0, 0.0).max_dimension(), 1);
        assert_eq!(Vec4::new(3.0, 0.0, 1.0, 2.0).max_dimension(), 0);
        assert_eq!(Vec4::new(3.0, 0.0, 2.0, 1.0).max_dimension(), 0);
        assert_eq!(Vec4::new(3.0, 1.0, 0.0, 2.0).max_dimension(), 0);
        assert_eq!(Vec4::new(3.0, 1.0, 2.0, 0.0).max_dimension(), 0);
        assert_eq!(Vec4::new(3.0, 2.0, 0.0, 1.0).max_dimension(), 0);
        assert_eq!(Vec4::new(3.0, 2.0, 1.0, 0.0).max_dimension(), 0);
    }

    #[test]
    fn permutation() {
        assert_eq!(Vec2::new(2.0, 3.0).permuted(1, 0), Vec2::new(3.0, 2.0));
        assert_eq!(
            Vec3::new(3.0, 4.0, 5.0).permuted(1, 2, 0),
            Vec3::new(4.0, 5.0, 3.0)
        );
        assert_eq!(
            Vec4::new(4.0, 5.0, 6.0, 7.0).permuted(1, 2, 3, 0),
            Vec4::new(5.0, 6.0, 7.0, 4.0)
        );
    }

    #[test]
    fn negation() {
        assert_eq!(-Vec2::new(1, 2), Vec2::new(-1, -2));
        assert_eq!(-Vec3::new(1, 2, 3), Vec3::new(-1, -2, -3));
        assert_eq!(-Vec4::new(1, 2, 3, 4), Vec4::new(-1, -2, -3, -4));
    }

    #[test]
    fn add() {
        assert_eq!(Vec2::new(1, 2) + Vec2::new(4, 6), Vec2::new(5, 8));
        assert_eq!(Vec3::new(1, 2, 3) + Vec3::new(4, 6, 7), Vec3::new(5, 8, 10));
        assert_eq!(
            Vec4::new(1, 2, 3, 4) + Vec4::new(5, 7, 9, 10),
            Vec4::new(6, 9, 12, 14)
        );
        assert_eq!(Vec2::new(1, 2) + 3, Vec2::new(4, 5));
        assert_eq!(Vec3::new(1, 2, 3) + 4, Vec3::new(5, 6, 7));
        assert_eq!(Vec4::new(1, 2, 3, 4) + 5, Vec4::new(6, 7, 8, 9));
    }

    #[test]
    fn add_assign() {
        let mut v = Vec2::new(1, 2);
        v += Vec2::new(4, 6);
        assert_eq!(v, Vec2::new(5, 8));

        let mut v = Vec3::new(1, 2, 3);
        v += Vec3::new(4, 6, 7);
        assert_eq!(v, Vec3::new(5, 8, 10));

        let mut v = Vec4::new(1, 2, 3, 4);
        v += Vec4::new(5, 7, 9, 10);
        assert_eq!(v, Vec4::new(6, 9, 12, 14));

        let mut v = Vec2::new(1, 2);
        v += 3;
        assert_eq!(v, Vec2::new(4, 5));

        let mut v = Vec3::new(1, 2, 3);
        v += 4;
        assert_eq!(v, Vec3::new(5, 6, 7));

        let mut v = Vec4::new(1, 2, 3, 4);
        v += 5;
        assert_eq!(v, Vec4::new(6, 7, 8, 9));
    }

    #[test]
    fn sub() {
        assert_eq!(Vec2::new(5, 5) - Vec2::new(1, 2), Vec2::new(4, 3));
        assert_eq!(Vec3::new(7, 7, 7) - Vec3::new(1, 2, 3), Vec3::new(6, 5, 4));
        assert_eq!(
            Vec4::new(9, 9, 9, 9) - Vec4::new(1, 2, 3, 4),
            Vec4::new(8, 7, 6, 5)
        );
        assert_eq!(Vec2::new(3, 2) - 2, Vec2::new(1, 0));
        assert_eq!(Vec3::new(7, 6, 5) - 4, Vec3::new(3, 2, 1));
        assert_eq!(Vec4::new(9, 8, 7, 6) - 5, Vec4::new(4, 3, 2, 1));
    }

    #[test]
    fn sub_assign() {
        let mut v = Vec2::new(5, 5);
        v -= Vec2::new(1, 2);
        assert_eq!(v, Vec2::new(4, 3));

        let mut v = Vec3::new(7, 7, 7);
        v -= Vec3::new(1, 2, 3);
        assert_eq!(v, Vec3::new(6, 5, 4));

        let mut v = Vec4::new(9, 9, 9, 9);
        v -= Vec4::new(1, 2, 3, 4);
        assert_eq!(v, Vec4::new(8, 7, 6, 5));

        let mut v = Vec2::new(3, 2);
        v -= 2;
        assert_eq!(v, Vec2::new(1, 0));

        let mut v = Vec3::new(7, 6, 5);
        v -= 4;
        assert_eq!(v, Vec3::new(3, 2, 1));

        let mut v = Vec4::new(9, 8, 7, 6);
        v -= 5;
        assert_eq!(v, Vec4::new(4, 3, 2, 1));
    }

    #[test]
    fn mul() {
        assert_eq!(Vec2::new(2, 3) * 4, Vec2::new(8, 12));
        assert_eq!(Vec3::new(2, 3, 4) * 5, Vec3::new(10, 15, 20));
        assert_eq!(Vec4::new(2, 3, 4, 5) * 6, Vec4::new(12, 18, 24, 30));
    }

    #[test]
    fn mul_assign() {
        let mut v = Vec2::new(2, 3);
        v *= 4;
        assert_eq!(v, Vec2::new(8, 12));

        let mut v = Vec3::new(2, 3, 4);
        v *= 5;
        assert_eq!(v, Vec3::new(10, 15, 20));

        let mut v = Vec4::new(2, 3, 4, 5);
        v *= 6;
        assert_eq!(v, Vec4::new(12, 18, 24, 30));
    }

    #[test]
    fn div() {
        assert_eq!(Vec2::new(8, 12) / 4, Vec2::new(2, 3));
        assert_eq!(Vec3::new(10, 15, 20) / 5, Vec3::new(2, 3, 4));
        assert_eq!(Vec4::new(12, 18, 24, 30) / 6, Vec4::new(2, 3, 4, 5));
    }

    #[test]
    fn div_assign() {
        let mut v = Vec2::new(8, 12);
        v /= 4;
        assert_eq!(v, Vec2::new(2, 3));

        let mut v = Vec3::new(10, 15, 20);
        v /= 5;
        assert_eq!(v, Vec3::new(2, 3, 4));

        let mut v = Vec4::new(12, 18, 24, 30);
        v /= 6;
        assert_eq!(v, Vec4::new(2, 3, 4, 5));
    }
}
