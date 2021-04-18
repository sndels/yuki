use crate::math::{Normal, Point3, Vec3};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Interactions#SurfaceInteraction

// Info for a point on a surface
#[derive(Clone)]
pub struct SurfaceInteraction {
    /// World position
    pub p: Point3<f32>,
    /// View direction
    pub v: Vec3<f32>,
    /// Surface normal
    pub n: Normal<f32>,
    /// Diffuse surface color
    pub albedo: Vec3<f32>,
}
