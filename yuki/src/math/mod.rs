mod bounds;
mod common;
mod matrix;
mod normal;
mod point;
mod ray;
mod transform;
pub mod transforms;
mod vector;

pub use bounds::{Bounds2, Bounds3};
pub use common::ValueType;
pub use matrix::Matrix4x4;
pub use normal::Normal;
pub use point::{Point2, Point3};
pub use ray::Ray;
pub use transform::Transform;
pub use vector::{Vec2, Vec3, Vec4};

// From https://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Vectors#CoordinateSystemfromaVector
/// Creates perpendicular vectors for `v`.
///
/// `v` is expecte to be normalized.
pub fn coordinate_system<T: common::FloatValueType>(v: Vec3<T>) -> (Vec3<T>, Vec3<T>) {
    let v1 = if v.x.abs() > v.y.abs() {
        Vec3::new(-v.z, T::zero(), v.x) / (v.x * v.x + v.z * v.z).sqrt()
    } else {
        Vec3::new(T::zero(), v.z, -v.y) / (v.y * v.y + v.z + v.z)
    };
    let v2 = v.cross(v1);
    (v1, v2)
}
