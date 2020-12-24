use num::cast::{FromPrimitive, ToPrimitive};
use num::traits::{Float, Num, Signed};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use crate::helpers::{Maxi, Mini};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html

/// Generic two-component vector
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Vec2<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    pub x: T,
    pub y: T,
}

/// Generic three-component vector
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    pub x: T,
    pub y: T,
    pub z: T,
}

/// Generic four-component vector
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

impl<T> Vec2<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    /// Constructs a new vector.
    ///
    /// Has a debug assert that checks for NaNs.
    pub fn new(x: T, y: T) -> Vec2<T> {
        let v = Vec2 { x, y };
        debug_assert!(!v.has_nans());
        v
    }

    /// Returns `true` if any component is NaN.
    pub fn has_nans(&self) -> bool {
        // Cast to f64 since it is currently the largest floating point type
        (self.x.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
            || (self.y.to_f64().unwrap_or(f64::NAN) as f64).is_nan()
    }

    /// Returns the dot product of the two vectors.
    pub fn dot(&self, other: &Vec2<T>) -> T {
        self.x * other.x + self.y * other.y
    }

    /// Returns the vector's squared length.
    pub fn len_sqr(&self) -> T {
        self.dot(self)
    }

    /// Returns the vector's length.
    pub fn len(&self) -> T {
        T::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
    }

    /// Returns the normalized vector.
    pub fn normalized(&self) -> Vec2<T> {
        *self / self.len()
    }

    /// Returns the component-wise minimum of the two vectors.
    pub fn min(&self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: self.x.mini(other.x),
            y: self.y.mini(other.y),
        }
    }

    /// Returns the component-wise maximum of the two vectors.
    pub fn max(&self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: self.x.maxi(other.x),
            y: self.y.maxi(other.y),
        }
    }

    /// Returns the value of the minumum component.
    pub fn min_comp(&self) -> T {
        self.x.mini(self.y)
    }

    /// Returns the value of the maximum component.
    pub fn max_comp(&self) -> T {
        self.x.maxi(self.y)
    }

    /// Returns the index of the maximum component.
    pub fn max_dimension(&self) -> usize {
        if self.x > self.y {
            0
        } else {
            1
        }
    }

    /// Returns the vector permutation defined by the indices.
    pub fn permuted(&self, x: usize, y: usize) -> Vec2<T> {
        Vec2 {
            x: self[x],
            y: self[y],
        }
    }
}

impl<T> Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    /// Constructs a new vector.
    ///
    /// Has a debug assert that checks for NaNs.
    pub fn new(x: T, y: T, z: T) -> Vec3<T> {
        let v = Vec3 { x, y, z };
        debug_assert!(!v.has_nans());
        v
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
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Returns the vector's squared length.
    pub fn len_sqr(&self) -> T {
        self.dot(self)
    }

    /// Returns the vector's length.
    pub fn len(&self) -> T {
        T::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
    }

    /// Returns the normalized vector.
    pub fn normalized(&self) -> Vec3<T> {
        *self / self.len()
    }

    /// Returns the component-wise minimum of the two vectors.
    pub fn min(&self, other: Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: self.x.mini(other.x),
            y: self.y.mini(other.y),
            z: self.z.mini(other.z),
        }
    }

    /// Returns the component-wise maximum of the two vectors.
    pub fn max(&self, other: Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: self.x.maxi(other.x),
            y: self.y.maxi(other.y),
            z: self.z.maxi(other.z),
        }
    }

    /// Returns the value of the minumum component.
    pub fn min_comp(&self) -> T {
        self.x.mini(self.y.mini(self.z))
    }

    /// Returns the value of the maximum component.
    pub fn max_comp(&self) -> T {
        self.x.maxi(self.y.maxi(self.z))
    }

    /// Returns the index of the maximum component.
    pub fn max_dimension(&self) -> usize {
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
        Vec3 {
            x: self[x],
            y: self[y],
            z: self[z],
        }
    }
}

impl<T> Vec3<T>
where
    T: Float + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    /// Returns the cross product of the two vectors.
    pub fn cross(&self, other: Vec3<T>) -> Vec3<T> {
        //!
        //! Always uses `f64` internally to avoid errors on "catastrophic cancellation".
        //! See pbrt [2.2.1](http://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors.html#DotandCrossProduct) for details
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    /// Constructs a new vector.
    ///
    /// Has a debug assert that checks for NaNs.
    pub fn new(x: T, y: T, z: T, w: T) -> Vec4<T> {
        let v = Vec4 { x, y, z, w };
        debug_assert!(!v.has_nans());
        v
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
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Returns the vector's squared length.
    pub fn len_sqr(&self) -> T {
        self.dot(self)
    }

    /// Returns the vector's length.
    pub fn len(&self) -> T {
        T::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
    }

    /// Returns the normalized vector.
    pub fn normalized(&self) -> Vec4<T> {
        *self / self.len()
    }

    /// Returns the component-wise minimum of the two vectors.
    pub fn min(&self, other: Vec4<T>) -> Vec4<T> {
        Vec4 {
            x: self.x.mini(other.x),
            y: self.y.mini(other.y),
            z: self.z.mini(other.z),
            w: self.w.mini(other.w),
        }
    }

    /// Returns the component-wise maximum of the two vectors.
    pub fn max(&self, other: Vec4<T>) -> Vec4<T> {
        Vec4 {
            x: self.x.maxi(other.x),
            y: self.y.maxi(other.y),
            z: self.z.maxi(other.z),
            w: self.w.maxi(other.w),
        }
    }

    /// Returns the value of the minumum component.
    pub fn min_comp(&self) -> T {
        let a = self.x.mini(self.y);
        let b = self.z.mini(self.w);
        a.mini(b)
    }

    /// Returns the value of the maximum component.
    pub fn max_comp(&self) -> T {
        let a = self.x.maxi(self.y);
        let b = self.z.maxi(self.w);
        a.maxi(b)
    }

    /// Returns the index of the maximum component.
    pub fn max_dimension(&self) -> usize {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = T;

    fn index(&self, component: usize) -> &Self::Output {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = T;

    fn index(&self, component: usize) -> &Self::Output {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = T;

    fn index(&self, component: usize) -> &Self::Output {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn index_mut(&mut self, component: usize) -> &mut Self::Output {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn index_mut(&mut self, component: usize) -> &mut Self::Output {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn index_mut(&mut self, component: usize) -> &mut Self::Output {
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

impl<T> Neg for Vec2<T>
where
    T: Signed + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Vec2<T>;

    fn neg(self) -> Vec2<T> {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl<T> Neg for Vec3<T>
where
    T: Signed + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Vec3<T>;

    fn neg(self) -> Vec3<T> {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl<T> Neg for Vec4<T>
where
    T: Signed + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Vec4<T>;

    fn neg(self) -> Vec4<T> {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T> Add for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl<T> Add for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl<T> AddAssign for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl<T> AddAssign for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl<T> Add<T> for Vec2<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn add(self, other: T) -> Self {
        Self {
            x: self.x + other,
            y: self.y + other,
        }
    }
}

impl<T> Add<T> for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn add(self, other: T) -> Self {
        Self {
            x: self.x + other,
            y: self.y + other,
            z: self.z + other,
        }
    }
}

impl<T> Add<T> for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn add(self, other: T) -> Self {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn add_assign(&mut self, other: T) {
        *self = *self + other;
    }
}

impl<T> AddAssign<T> for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn add_assign(&mut self, other: T) {
        *self = *self + other;
    }
}

impl<T> AddAssign<T> for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn add_assign(&mut self, other: T) {
        *self = *self + other;
    }
}

impl<T> Sub for Vec2<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<T> Sub for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl<T> Sub for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl<T> SubAssign for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl<T> SubAssign for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl<T> Mul<T> for Vec2<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn mul(self, other: T) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl<T> Mul<T> for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn mul(self, other: T) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}

impl<T> Mul<T> for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn mul(self, other: T) -> Self {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn mul_assign(&mut self, other: T) {
        *self = *self * other;
    }
}

impl<T> MulAssign<T> for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn mul_assign(&mut self, other: T) {
        *self = *self * other;
    }
}

impl<T> MulAssign<T> for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn mul_assign(&mut self, other: T) {
        *self = *self * other;
    }
}

impl<T> Div<T> for Vec2<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn div(self, other: T) -> Self {
        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl<T> Div<T> for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn div(self, other: T) -> Self {
        Self {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
        }
    }
}

impl<T> Div<T> for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    type Output = Self;

    fn div(self, other: T) -> Self {
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
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn div_assign(&mut self, other: T) {
        *self = *self / other;
    }
}

impl<T> DivAssign<T> for Vec3<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn div_assign(&mut self, other: T) {
        *self = *self / other;
    }
}

impl<T> DivAssign<T> for Vec4<T>
where
    T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy,
{
    fn div_assign(&mut self, other: T) {
        *self = *self / other;
    }
}

pub type Vec2f = Vec2<f32>;
pub type Vec3f = Vec3<f32>;
pub type Vec4f = Vec4<f32>;

pub type Vec2i = Vec2<i32>;
pub type Vec3i = Vec3<i32>;
pub type Vec4i = Vec4<i32>;

pub type Vec2u = Vec2<u32>;
pub type Vec3u = Vec3<u32>;
pub type Vec4u = Vec4<u32>;
