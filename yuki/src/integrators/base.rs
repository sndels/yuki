use crate::{
    math::{Ray, Vec3},
    scene::Scene,
};

pub struct RadianceResult {
    pub li: Vec3<f32>,
    pub ray_scene_intersections: usize,
}

/// Generic interface that needs to be implemented by all Integrators.
pub trait IntegratorBase {
    /// Evaluates the incoming radiance along `ray`. Also returns the number of rays intersected with `scene`.
    fn li(&self, ray: Ray<f32>, scene: &Scene, depth: u32) -> RadianceResult;
}
