use std::ops::Mul;

use super::{
    bounds::Bounds3, common::FloatValueType, matrix::Matrix4x4, normal::Normal, point::Point3,
    ray::Ray, vector::Vec3,
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Geometry_and_Transforms/Transforms.html

#[derive(Clone, Debug, PartialEq)]
pub struct Transform<T>
where
    T: FloatValueType,
{
    m: Matrix4x4<T>,
    m_inv: Matrix4x4<T>,
}

impl<T> Transform<T>
where
    T: FloatValueType,
{
    /// Creates a new `Transform` from raw [Matrix4x4] rows.
    pub fn new(m: [[T; 4]; 4]) -> Self {
        let m = Matrix4x4::new(m);
        Self::new_m(m)
    }

    /// Creates a new `Transform` from a [Matrix4x4].
    pub fn new_m(m: Matrix4x4<T>) -> Self {
        let m_inv = m.inverted();
        Self::new_full(m, m_inv)
    }

    /// Creates a new `Transform` from a [Matrix4x4] and its inverse.
    pub fn new_full(m: Matrix4x4<T>, m_inv: Matrix4x4<T>) -> Self {
        debug_assert!(!m.has_nans());
        debug_assert!(!m_inv.has_nans());
        Self { m, m_inv }
    }

    /// Creates a new identity `Transform`.
    pub fn default() -> Self {
        let m = Matrix4x4::identity();
        Self::new_full(m, m)
    }

    /// Returns a reference to the [Matrix4x4] of this `Transformation`.
    pub fn m(&self) -> &Matrix4x4<T> {
        &self.m
    }

    /// Returns a reference to the inverse [Matrix4x4] of this `Transformation`.
    pub fn m_inv(&self) -> &Matrix4x4<T> {
        &self.m_inv
    }

    /// Returns the inverse of this `Transform`.
    pub fn inverted(&self) -> Self {
        Self::new_full(self.m_inv, self.m)
    }

    /// Returns the transpose of this `Transform`.
    pub fn transposed(&self) -> Self {
        Self::new_full(self.m.transposed(), self.m_inv.transposed())
    }

    /// Checks if this `Transform` is the identity transform.
    pub fn is_identity(&self) -> bool {
        self.m.m[0][0] == T::one()
            && self.m.m[0][1] == T::zero()
            && self.m.m[0][2] == T::zero()
            && self.m.m[0][3] == T::zero()
            && self.m.m[1][0] == T::zero()
            && self.m.m[1][1] == T::one()
            && self.m.m[1][2] == T::zero()
            && self.m.m[1][3] == T::zero()
            && self.m.m[2][0] == T::zero()
            && self.m.m[2][1] == T::zero()
            && self.m.m[2][2] == T::one()
            && self.m.m[2][3] == T::zero()
            && self.m.m[3][0] == T::zero()
            && self.m.m[3][1] == T::zero()
            && self.m.m[3][2] == T::zero()
            && self.m.m[3][3] == T::one()
    }

    /// Checks if this `Transform` swaps the handedness of the coordinate system.
    pub fn swaps_handedness(&self) -> bool {
        let m = &self.m.m;
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        det < T::zero()
    }
}

impl<'a, T> Mul<Vec3<T>> for &'a Transform<T>
where
    T: FloatValueType,
{
    type Output = Vec3<T>;

    fn mul(self, other: Vec3<T>) -> Vec3<T> {
        let m = &self.m.m;
        let x = other.x;
        let y = other.y;
        let z = other.z;
        Vec3::new(
            m[0][0] * x + m[0][1] * y + m[0][2] * z,
            m[1][0] * x + m[1][1] * y + m[1][2] * z,
            m[2][0] * x + m[2][1] * y + m[2][2] * z,
        )
    }
}

impl<'a, T> Mul<Point3<T>> for &'a Transform<T>
where
    T: FloatValueType,
{
    type Output = Point3<T>;

    fn mul(self, other: Point3<T>) -> Point3<T> {
        let m = &self.m.m;
        let x = other.x;
        let y = other.y;
        let z = other.z;
        let xp = m[0][0] * x + m[0][1] * y + m[0][2] * z + m[0][3];
        let yp = m[1][0] * x + m[1][1] * y + m[1][2] * z + m[1][3];
        let zp = m[2][0] * x + m[2][1] * y + m[2][2] * z + m[2][3];
        let wp = m[3][0] * x + m[3][1] * y + m[3][2] * z + m[3][3];
        if wp == T::one() {
            Point3::new(xp, yp, zp)
        } else {
            Point3::new(xp, yp, zp) / wp
        }
    }
}

impl<'a, T> Mul<Normal<T>> for &'a Transform<T>
where
    T: FloatValueType,
{
    type Output = Normal<T>;

    fn mul(self, other: Normal<T>) -> Normal<T> {
        let m_inv = &self.m_inv.m;
        let x = other.x;
        let y = other.y;
        let z = other.z;
        // Transpose inverse matrix through accesses
        Normal::new(
            m_inv[0][0] * x + m_inv[1][0] * y + m_inv[2][0] * z,
            m_inv[0][1] * x + m_inv[1][1] * y + m_inv[2][1] * z,
            m_inv[0][2] * x + m_inv[1][2] * y + m_inv[2][2] * z,
        )
    }
}

impl<'a, T> Mul<Ray<T>> for &'a Transform<T>
where
    T: FloatValueType,
{
    type Output = Ray<T>;

    fn mul(self, other: Ray<T>) -> Ray<T> {
        Ray::new(
            self * other.o, // TODO: Offset to error bound
            self * other.d,
            other.t_max,
        )
    }
}

impl<'a, T> Mul<Bounds3<T>> for &'a Transform<T>
where
    T: FloatValueType,
{
    type Output = Bounds3<T>;

    fn mul(self, other: Bounds3<T>) -> Bounds3<T> {
        let mi = other.p_min;
        let ma = other.p_max;

        // TODO: This could be done much more efficently
        let mut ret = Bounds3::default();
        ret = ret.union_p(self * mi);
        ret = ret.union_p(self * Point3::new(ma.x, mi.y, mi.z));
        ret = ret.union_p(self * Point3::new(mi.x, ma.y, mi.z));
        ret = ret.union_p(self * Point3::new(mi.x, mi.y, ma.z));
        ret = ret.union_p(self * Point3::new(ma.x, ma.y, mi.z));
        ret = ret.union_p(self * Point3::new(ma.x, mi.y, ma.z));
        ret = ret.union_p(self * Point3::new(mi.x, ma.y, ma.z));
        ret = ret.union_p(self * ma);
        ret
    }
}

impl<'a, 'b, T> Mul<&'b Transform<T>> for &'a Transform<T>
where
    T: FloatValueType,
{
    type Output = Transform<T>;

    fn mul(self, other: &Transform<T>) -> Transform<T> {
        Transform::new_full(&self.m * &other.m, &other.m_inv * &self.m_inv)
    }
}

/// Creates a new `Transform` that is a translation by `delta`.
pub fn translation<T>(delta: Vec3<T>) -> Transform<T>
where
    T: FloatValueType,
{
    let m = Matrix4x4::new([
        [T::one(), T::zero(), T::zero(), delta.x],
        [T::zero(), T::one(), T::zero(), delta.y],
        [T::zero(), T::zero(), T::one(), delta.z],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);
    let m_inv = Matrix4x4::new([
        [T::one(), T::zero(), T::zero(), -delta.x],
        [T::zero(), T::one(), T::zero(), -delta.y],
        [T::zero(), T::zero(), T::one(), -delta.z],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);

    Transform::new_full(m, m_inv)
}

/// Creates a new `Transform` that is a scaling by `x`, `y` and `z`.
pub fn scale<T>(x: T, y: T, z: T) -> Transform<T>
where
    T: FloatValueType,
{
    let m = Matrix4x4::new([
        [x, T::zero(), T::zero(), T::zero()],
        [T::zero(), y, T::zero(), T::zero()],
        [T::zero(), T::zero(), z, T::zero()],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);
    let m_inv = Matrix4x4::new([
        [T::one() / x, T::zero(), T::zero(), T::zero()],
        [T::zero(), T::one() / y, T::zero(), T::zero()],
        [T::zero(), T::zero(), T::one() / z, T::zero()],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);

    Transform::new_full(m, m_inv)
}

/// Creates a new `Transform` that is a rotation of `theta` radians around the x-axis.
pub fn rotation_x<T>(theta: T) -> Transform<T>
where
    T: FloatValueType,
{
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();
    let m = Matrix4x4::new([
        [T::one(), T::zero(), T::zero(), T::zero()],
        [T::zero(), cos_theta, -sin_theta, T::zero()],
        [T::zero(), sin_theta, cos_theta, T::zero()],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);

    Transform::new_full(m, m.transposed())
}

/// Creates a new `Transform` that is a rotation of `theta` radians around the y-axis.
pub fn rotation_y<T>(theta: T) -> Transform<T>
where
    T: FloatValueType,
{
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();
    let m = Matrix4x4::new([
        [cos_theta, T::zero(), sin_theta, T::zero()],
        [T::zero(), T::one(), T::zero(), T::zero()],
        [-sin_theta, T::zero(), cos_theta, T::zero()],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);

    Transform::new_full(m, m.transposed())
}

/// Creates a new `Transform` that is a rotation of `theta` radians around the z-axis.
pub fn rotation_z<T>(theta: T) -> Transform<T>
where
    T: FloatValueType,
{
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();
    let m = Matrix4x4::new([
        [cos_theta, -sin_theta, T::zero(), T::zero()],
        [sin_theta, cos_theta, T::zero(), T::zero()],
        [T::zero(), T::zero(), T::one(), T::zero()],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);

    Transform::new_full(m, m.transposed())
}

/// Creates a new `Transform` that is a rotation of `theta` radians around `axis`.
pub fn rotation<T>(theta: T, axis: Vec3<T>) -> Transform<T>
where
    T: FloatValueType,
{
    let a = axis.normalized();
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();
    let m = Matrix4x4::new([
        [
            a.x * a.x + (T::one() - a.x * a.x) * cos_theta,
            a.x * a.y * (T::one() - cos_theta) - a.z * sin_theta,
            a.x * a.z * (T::one() - cos_theta) + a.y * sin_theta,
            T::zero(),
        ],
        [
            a.x * a.y * (T::one() - cos_theta) + a.z * sin_theta,
            a.y * a.y + (T::one() - a.y * a.y) * cos_theta,
            a.y * a.z * (T::one() - cos_theta) - a.x * sin_theta,
            T::zero(),
        ],
        [
            a.x * a.z * (T::one() - cos_theta) - a.y * sin_theta,
            a.y * a.z * (T::one() - cos_theta) + a.x * sin_theta,
            a.z * a.z + (T::one() - a.z * a.z) * cos_theta,
            T::zero(),
        ],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);

    Transform::new_full(m, m.transposed())
}

/// Creates a world_to_camera `Transform` with the camera at `pos` looking at `target` with `up` as the up vector.
pub fn look_at<T>(pos: Point3<T>, target: Point3<T>, up: Vec3<T>) -> Transform<T>
where
    T: FloatValueType,
{
    let dir = (target - pos).normalized();
    let right = up.normalized().cross(dir).normalized();
    let new_up = dir.cross(right);
    let camera_to_world = Matrix4x4::new([
        [right.x, new_up.x, dir.x, pos.x],
        [right.y, new_up.y, dir.y, pos.y],
        [right.z, new_up.z, dir.z, pos.z],
        [T::zero(), T::zero(), T::zero(), T::one()],
    ]);

    Transform::new_full(camera_to_world.inverted(), camera_to_world)
}
