use num::traits::{Float, Signed};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use crate::helpers::ValueType;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html

/// Generic two-component vector
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec2<T>
where
    T: ValueType,
{
    pub x: T,
    pub y: T,
}

/// Generic three-component vector
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec3<T>
where
    T: ValueType,
{
    pub x: T,
    pub y: T,
    pub z: T,
}

/// Generic four-component vector
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec4<T>
where
    T: ValueType,
{
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

impl<T> Vec2<T>
where
    T: ValueType,
{
    /// Constructs a new vector.
    ///
    /// Has a debug assert that checks for NaNs.
    pub fn new(x: T, y: T) -> Vec2<T> {
        let v = Vec2 { x, y };
        debug_assert!(!v.has_nans());
        v
    }

    /// Constructs a new vector of 0s.
    pub fn zeros() -> Vec2<T> {
        Vec2 {
            x: T::zero(),
            y: T::zero(),
        }
    }

    /// Constructs a new vector of 1s.
    pub fn ones() -> Vec2<T> {
        Vec2 {
            x: T::one(),
            y: T::one(),
        }
    }

    /// Returns `true` if any component is NaN.
    pub fn has_nans(&self) -> bool {
        // Cast to f64 since it is currently the largest floating point type
        (self.x.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
            || (self.y.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
    }

    /// Returns the dot product of the two vectors.
    pub fn dot(&self, other: &Vec2<T>) -> T {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        self.x * other.x + self.y * other.y
    }

    /// Returns the vector's squared length.
    pub fn len_sqr(&self) -> T {
        debug_assert!(!self.has_nans());

        self.dot(self)
    }

    /// Returns the vector's length.
    pub fn len(&self) -> T {
        debug_assert!(!self.has_nans());

        T::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
    }

    /// Returns the normalized vector.
    pub fn normalized(&self) -> Vec2<T> {
        debug_assert!(!self.has_nans());

        *self / self.len()
    }

    /// Returns the component-wise minimum of the two vectors.
    pub fn min(&self, other: Vec2<T>) -> Vec2<T> {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Vec2 {
            x: self.x.mini(other.x),
            y: self.y.mini(other.y),
        }
    }

    /// Returns the component-wise maximum of the two vectors.
    pub fn max(&self, other: Vec2<T>) -> Vec2<T> {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Vec2 {
            x: self.x.maxi(other.x),
            y: self.y.maxi(other.y),
        }
    }

    /// Returns the value of the minumum component.
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.mini(self.y)
    }

    /// Returns the value of the maximum component.
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.maxi(self.y)
    }

    /// Returns the index of the maximum component.
    pub fn max_dimension(&self) -> usize {
        debug_assert!(!self.has_nans());

        if self.x > self.y {
            0
        } else {
            1
        }
    }

    /// Returns the vector permutation defined by the indices.
    pub fn permuted(&self, x: usize, y: usize) -> Vec2<T> {
        debug_assert!(!self.has_nans());

        Vec2 {
            x: self[x],
            y: self[y],
        }
    }
}

impl<T> Vec3<T>
where
    T: ValueType,
{
    /// Constructs a new vector.
    ///
    /// Has a debug assert that checks for NaNs.
    pub fn new(x: T, y: T, z: T) -> Vec3<T> {
        let v = Vec3 { x, y, z };
        debug_assert!(!v.has_nans());
        v
    }

    /// Constructs a new vector of 0s.
    pub fn zeros() -> Vec3<T> {
        Vec3 {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
        }
    }

    /// Constructs a new vector of 1s.
    pub fn ones() -> Vec3<T> {
        Vec3 {
            x: T::one(),
            y: T::one(),
            z: T::one(),
        }
    }

    /// Returns `true` if any component is NaN.
    pub fn has_nans(&self) -> bool {
        // Cast to f64 since it is currently the largest floating point type
        (self.x.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
            || (self.y.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
            || (self.z.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
    }

    /// Returns the dot product of the two vectors.
    pub fn dot(&self, other: &Vec3<T>) -> T {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Returns the vector's squared length.
    pub fn len_sqr(&self) -> T {
        debug_assert!(!self.has_nans());

        self.dot(self)
    }

    /// Returns the vector's length.
    pub fn len(&self) -> T {
        debug_assert!(!self.has_nans());

        T::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
    }

    /// Returns the normalized vector.
    pub fn normalized(&self) -> Vec3<T> {
        debug_assert!(!self.has_nans());

        *self / self.len()
    }

    /// Returns the component-wise minimum of the two vectors.
    pub fn min(&self, other: Vec3<T>) -> Vec3<T> {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Vec3 {
            x: self.x.mini(other.x),
            y: self.y.mini(other.y),
            z: self.z.mini(other.z),
        }
    }

    /// Returns the component-wise maximum of the two vectors.
    pub fn max(&self, other: Vec3<T>) -> Vec3<T> {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Vec3 {
            x: self.x.maxi(other.x),
            y: self.y.maxi(other.y),
            z: self.z.maxi(other.z),
        }
    }

    /// Returns the value of the minumum component.
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.mini(self.y.mini(self.z))
    }

    /// Returns the value of the maximum component.
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        self.x.maxi(self.y.maxi(self.z))
    }

    /// Returns the index of the maximum component.
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

    /// Returns the vector permutation defined by the indices.
    pub fn permuted(&self, x: usize, y: usize, z: usize) -> Vec3<T> {
        debug_assert!(!self.has_nans());

        Vec3 {
            x: self[x],
            y: self[y],
            z: self[z],
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
    pub fn cross(&self, other: Vec3<T>) -> Vec3<T> {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        let v1x = self.x.to_f64().unwrap_or(f64::NAN);
        let v1y = self.y.to_f64().unwrap_or(f64::NAN);
        let v1z = self.z.to_f64().unwrap_or(f64::NAN);
        let v2x = other.x.to_f64().unwrap_or(f64::NAN);
        let v2y = other.y.to_f64().unwrap_or(f64::NAN);
        let v2z = other.z.to_f64().unwrap_or(f64::NAN);
        Vec3 {
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
    /// Constructs a new vector.
    ///
    /// Has a debug assert that checks for NaNs.
    pub fn new(x: T, y: T, z: T, w: T) -> Vec4<T> {
        let v = Vec4 { x, y, z, w };
        debug_assert!(!v.has_nans());
        v
    }

    /// Constructs a new vector of 0s.
    pub fn zeros() -> Vec4<T> {
        Vec4 {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
            w: T::zero(),
        }
    }

    /// Constructs a new vector of 1s.
    pub fn ones() -> Vec4<T> {
        Vec4 {
            x: T::one(),
            y: T::one(),
            z: T::one(),
            w: T::one(),
        }
    }

    /// Returns `true` if any component is NaN.
    pub fn has_nans(&self) -> bool {
        // Cast to f64 since it is currently the largest floating point type
        (self.x.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
            || (self.y.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
            || (self.z.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
            || (self.w.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
    }

    /// Returns the dot product of the two vectors.
    pub fn dot(&self, other: &Vec4<T>) -> T {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Returns the vector's squared length.
    pub fn len_sqr(&self) -> T {
        debug_assert!(!self.has_nans());

        self.dot(self)
    }

    /// Returns the vector's length.
    pub fn len(&self) -> T {
        debug_assert!(!self.has_nans());

        T::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
    }

    /// Returns the normalized vector.
    pub fn normalized(&self) -> Vec4<T> {
        debug_assert!(!self.has_nans());

        *self / self.len()
    }

    /// Returns the component-wise minimum of the two vectors.
    pub fn min(&self, other: Vec4<T>) -> Vec4<T> {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Vec4 {
            x: self.x.mini(other.x),
            y: self.y.mini(other.y),
            z: self.z.mini(other.z),
            w: self.w.mini(other.w),
        }
    }

    /// Returns the component-wise maximum of the two vectors.
    pub fn max(&self, other: Vec4<T>) -> Vec4<T> {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Vec4 {
            x: self.x.maxi(other.x),
            y: self.y.maxi(other.y),
            z: self.z.maxi(other.z),
            w: self.w.maxi(other.w),
        }
    }

    /// Returns the value of the minumum component.
    pub fn min_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        let a = self.x.mini(self.y);
        let b = self.z.mini(self.w);
        a.mini(b)
    }

    /// Returns the value of the maximum component.
    pub fn max_comp(&self) -> T {
        debug_assert!(!self.has_nans());

        let a = self.x.maxi(self.y);
        let b = self.z.maxi(self.w);
        a.maxi(b)
    }

    /// Returns the index of the maximum component.
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

    /// Returns the vector permutation defined by the indices.
    pub fn permuted(&self, x: usize, y: usize, z: usize, w: usize) -> Vec4<T> {
        debug_assert!(!self.has_nans());

        Vec4 {
            x: self[x],
            y: self[y],
            z: self[z],
            w: self[w],
        }
    }
}

impl<T> Index<usize> for Vec2<T>
where
    T: ValueType,
{
    type Output = T;

    fn index(&self, component: usize) -> &Self::Output {
        debug_assert!(!self.has_nans());

        match component {
            0 => &self.x,
            1 => &self.y,
            _ => {
                panic!("Out of bounds Vec2 access with component {}", component);
            }
        }
    }
}

impl<T> Index<usize> for Vec3<T>
where
    T: ValueType,
{
    type Output = T;

    fn index(&self, component: usize) -> &Self::Output {
        debug_assert!(!self.has_nans());

        match component {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => {
                panic!("Out of bounds Vec2 access with component {}", component);
            }
        }
    }
}

impl<T> Index<usize> for Vec4<T>
where
    T: ValueType,
{
    type Output = T;

    fn index(&self, component: usize) -> &Self::Output {
        debug_assert!(!self.has_nans());

        match component {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            3 => &self.w,
            _ => {
                panic!("Out of bounds Vec2 access with component {}", component);
            }
        }
    }
}

impl<T> IndexMut<usize> for Vec2<T>
where
    T: ValueType,
{
    fn index_mut(&mut self, component: usize) -> &mut Self::Output {
        debug_assert!(!self.has_nans());

        match component {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => {
                panic!("Out of bounds Vec2 access with component {}", component);
            }
        }
    }
}

impl<T> IndexMut<usize> for Vec3<T>
where
    T: ValueType,
{
    fn index_mut(&mut self, component: usize) -> &mut Self::Output {
        debug_assert!(!self.has_nans());

        match component {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => {
                panic!("Out of bounds Vec2 access with component {}", component);
            }
        }
    }
}

impl<T> IndexMut<usize> for Vec4<T>
where
    T: ValueType,
{
    fn index_mut(&mut self, component: usize) -> &mut Self::Output {
        debug_assert!(!self.has_nans());

        match component {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            3 => &mut self.w,
            _ => {
                panic!("Out of bounds Vec2 access with component {}", component);
            }
        }
    }
}

impl<T> From<T> for Vec2<T>
where
    T: ValueType,
{
    fn from(v: T) -> Self {
        Self { x: v, y: v }
    }
}

impl<T> From<T> for Vec3<T>
where
    T: ValueType,
{
    fn from(v: T) -> Self {
        Self { x: v, y: v, z: v }
    }
}

impl<T> From<T> for Vec4<T>
where
    T: ValueType,
{
    fn from(v: T) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            w: v,
        }
    }
}

impl<T> Neg for Vec2<T>
where
    T: Signed + ValueType,
{
    type Output = Vec2<T>;

    fn neg(self) -> Vec2<T> {
        debug_assert!(!self.has_nans());

        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl<T> Neg for Vec3<T>
where
    T: Signed + ValueType,
{
    type Output = Vec3<T>;

    fn neg(self) -> Vec3<T> {
        debug_assert!(!self.has_nans());

        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl<T> Neg for Vec4<T>
where
    T: Signed + ValueType,
{
    type Output = Vec4<T>;

    fn neg(self) -> Vec4<T> {
        debug_assert!(!self.has_nans());

        Vec4 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
        }
    }
}

impl<T> Add for Vec2<T>
where
    T: ValueType,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T> Add for Vec3<T>
where
    T: ValueType,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl<T> Add for Vec4<T>
where
    T: ValueType,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl<T> AddAssign for Vec2<T>
where
    T: ValueType,
{
    fn add_assign(&mut self, other: Self) {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        *self = *self + other;
    }
}

impl<T> AddAssign for Vec3<T>
where
    T: ValueType,
{
    fn add_assign(&mut self, other: Self) {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        *self = *self + other;
    }
}

impl<T> AddAssign for Vec4<T>
where
    T: ValueType,
{
    fn add_assign(&mut self, other: Self) {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        *self = *self + other;
    }
}

impl<T> Add<T> for Vec2<T>
where
    T: ValueType,
{
    type Output = Self;

    fn add(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x + other,
            y: self.y + other,
        }
    }
}

impl<T> Add<T> for Vec3<T>
where
    T: ValueType,
{
    type Output = Self;

    fn add(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x + other,
            y: self.y + other,
            z: self.z + other,
        }
    }
}

impl<T> Add<T> for Vec4<T>
where
    T: ValueType,
{
    type Output = Self;

    fn add(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x + other,
            y: self.y + other,
            z: self.z + other,
            w: self.w + other,
        }
    }
}

impl<T> AddAssign<T> for Vec2<T>
where
    T: ValueType,
{
    fn add_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self + other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> AddAssign<T> for Vec3<T>
where
    T: ValueType,
{
    fn add_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self + other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> AddAssign<T> for Vec4<T>
where
    T: ValueType,
{
    fn add_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self + other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> Sub for Vec2<T>
where
    T: ValueType,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<T> Sub for Vec3<T>
where
    T: ValueType,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl<T> Sub for Vec4<T>
where
    T: ValueType,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl<T> SubAssign for Vec2<T>
where
    T: ValueType,
{
    fn sub_assign(&mut self, other: Self) {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        *self = *self - other;
    }
}

impl<T> SubAssign for Vec3<T>
where
    T: ValueType,
{
    fn sub_assign(&mut self, other: Self) {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        *self = *self - other;
    }
}

impl<T> SubAssign for Vec4<T>
where
    T: ValueType,
{
    fn sub_assign(&mut self, other: Self) {
        debug_assert!(!self.has_nans());
        debug_assert!(!other.has_nans());

        *self = *self - other;
    }
}

impl<T> Sub<T> for Vec2<T>
where
    T: ValueType,
{
    type Output = Self;

    fn sub(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x - other,
            y: self.y - other,
        }
    }
}

impl<T> Sub<T> for Vec3<T>
where
    T: ValueType,
{
    type Output = Self;

    fn sub(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x - other,
            y: self.y - other,
            z: self.z - other,
        }
    }
}

impl<T> Sub<T> for Vec4<T>
where
    T: ValueType,
{
    type Output = Self;

    fn sub(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x - other,
            y: self.y - other,
            z: self.z - other,
            w: self.w - other,
        }
    }
}

impl<T> SubAssign<T> for Vec2<T>
where
    T: ValueType,
{
    fn sub_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self - other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> SubAssign<T> for Vec3<T>
where
    T: ValueType,
{
    fn sub_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self - other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> SubAssign<T> for Vec4<T>
where
    T: ValueType,
{
    fn sub_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self - other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> Mul<T> for Vec2<T>
where
    T: ValueType,
{
    type Output = Self;

    fn mul(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl<T> Mul<T> for Vec3<T>
where
    T: ValueType,
{
    type Output = Self;

    fn mul(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}

impl<T> Mul<T> for Vec4<T>
where
    T: ValueType,
{
    type Output = Self;

    fn mul(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
            w: self.w * other,
        }
    }
}

impl<T> MulAssign<T> for Vec2<T>
where
    T: ValueType,
{
    fn mul_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self * other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> MulAssign<T> for Vec3<T>
where
    T: ValueType,
{
    fn mul_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self * other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> MulAssign<T> for Vec4<T>
where
    T: ValueType,
{
    fn mul_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self * other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> Div<T> for Vec2<T>
where
    T: ValueType,
{
    type Output = Self;

    fn div(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl<T> Div<T> for Vec3<T>
where
    T: ValueType,
{
    type Output = Self;

    fn div(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
        }
    }
}

impl<T> Div<T> for Vec4<T>
where
    T: ValueType,
{
    type Output = Self;

    fn div(self, other: T) -> Self {
        debug_assert!(!self.has_nans());

        Self {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
            w: self.w / other,
        }
    }
}

impl<T> DivAssign<T> for Vec2<T>
where
    T: ValueType,
{
    fn div_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self / other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> DivAssign<T> for Vec3<T>
where
    T: ValueType,
{
    fn div_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self / other;

        debug_assert!(!self.has_nans());
    }
}

impl<T> DivAssign<T> for Vec4<T>
where
    T: ValueType,
{
    fn div_assign(&mut self, other: T) {
        debug_assert!(!self.has_nans());

        *self = *self / other;

        debug_assert!(!self.has_nans());
    }
}

#[cfg(test)]
mod tests {
    // These tests are more about catching typos than rigorous verification
    // Thus, some assumptions (read: knowledge) of the implementation are made
    // and the tests should be updated according to potential implementation changes

    use approx::assert_abs_diff_eq;
    use std::panic;

    use crate::vector::Vec2;
    use crate::vector::Vec3;
    use crate::vector::Vec4;

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

        let v = Vec3::new(0.0, 1.0, 2.0);
        assert_eq!(v.x, v[0]);
        assert_eq!(v.y, v[1]);
        assert_eq!(v.z, v[2]);

        let v = Vec4::new(0.0, 1.0, 2.0, 3.0);
        assert_eq!(v.x, v[0]);
        assert_eq!(v.y, v[1]);
        assert_eq!(v.z, v[2]);
        assert_eq!(v.w, v[3]);

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
