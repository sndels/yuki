use crate::{
    hit::Hit,
    math::{bounds::Bounds3, ray::Ray},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Basic_Shape_Interface.html#Shape

pub trait Shape: Send + Sync {
    /// Intersects [Ray] with this object.
    fn intersect(&self, ray: Ray<f32>) -> Option<Hit>;
    /// Returns the world space AABB of the Shape
    fn world_bound(&self) -> Bounds3<f32>;
}
