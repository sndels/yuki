use super::{Integrator, RadianceResult};
use crate::{
    bvh::IntersectionResult,
    math::{Ray, Vec3},
    scene::Scene,
    shapes::Hit,
};

/// The first channel of returned color is the number of BVH intersections performed.
/// The second channel is the number of BVH node hits found.
/// The third channel is the number of BVH node hits found if the ray also hit scene geometry.
pub struct Normals {}

impl Integrator for Normals {
    fn li(&self, ray: Ray<f32>, scene: &Scene, _: u32, _: bool) -> RadianceResult {
        let IntersectionResult { hit, .. } = scene.bvh.intersect(ray);
        let ray_count = 1;

        let color = match hit {
            Some(Hit { si, .. }) => Vec3::from(si.n) / 2.0 + 0.5,
            None => Vec3::from(0.0),
        };

        RadianceResult {
            li: color,
            ray_scene_intersections: ray_count,
            rays: None,
        }
    }
}
