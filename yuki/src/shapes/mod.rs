mod mesh;
mod sphere;
mod triangle;

pub use mesh::Mesh;
pub use sphere::Sphere;
pub use triangle::Triangle;

use crate::{
    interaction::SurfaceInteraction,
    materials::Bsdf,
    math::{Bounds3, Ray},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Basic_Shape_Interface.html#Shape

pub struct Hit {
    pub t: f32,
    pub si: SurfaceInteraction,
    // Don't store in SurfaceInteraction like in pbrt to make lifetimes simpler
    // with allocator
    pub bsdf: Option<Bsdf>,
}

pub trait Shape: Send + Sync {
    /// Intersects [Ray] with this object.
    fn intersect(&self, ray: Ray<f32>) -> Option<Hit>;
    /// Returns the world space AABB of the Shape
    fn world_bound(&self) -> Bounds3<f32>;
    /// Returns `true` if the `Shape`s transform swaps coordinate system handedness
    fn transform_swaps_handedness(&self) -> bool;
    /// Computes the scattering functions for the intersection
    fn compute_scattering_functions(&self, si: &SurfaceInteraction) -> Bsdf;
}
