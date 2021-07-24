use super::{common::FloatValueType, matrix::Matrix4x4, point::Point3, vector::Vec3, Transform};

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

/// Creates a new `Transform` that is a rotation of euler angles `theta`.
pub fn rotation_euler<T>(theta: Vec3<T>) -> Transform<T>
where
    T: FloatValueType,
{
    &rotation_x(theta.x) * &(&rotation_y(theta.y) * &rotation_z(theta.z))
}

/// Creates a world-to-camera [`Transform`] with the camera at `pos` looking at `target` with `up` as the up vector.
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
