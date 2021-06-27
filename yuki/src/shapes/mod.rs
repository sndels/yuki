mod mesh;
mod sphere;
mod triangle;

pub use mesh::Mesh;
pub use sphere::Sphere;
pub use triangle::Triangle;

use crate::{
    interaction::SurfaceInteraction,
    math::{Bounds3, Ray},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Basic_Shape_Interface.html#Shape

pub struct Hit {
    pub t: f32,
    pub si: SurfaceInteraction,
}

pub trait Shape: Send + Sync {
    /// Intersects [Ray] with this object.
    fn intersect(&self, ray: Ray<f32>) -> Option<Hit>;
    /// Returns the world space AABB of the Shape
    fn world_bound(&self) -> Bounds3<f32>;
}
