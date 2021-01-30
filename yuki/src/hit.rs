use crate::math::{normal::Normal, point::Point3, vector::Vec3};

/// Info of a surface hit
pub struct Hit {
    /// Hit distance
    pub t: f32,
    /// World position
    pub p: Point3<f32>,
    /// View direction
    pub v: Vec3<f32>,
    /// Surface normal
    pub n: Normal<f32>,
    /// Diffuse surface color
    pub albedo: Vec3<f32>,
}
