use super::{Integrator, RadianceResult};
use crate::{
    bvh::IntersectionResult,
    math::{Ray, Spectrum},
    sampling::Sampler,
    scene::Scene,
};

use allocators::ScopedScratch;

/// The first channel of returned color is the number of BVH intersections performed.
/// The second channel is the number of BVH node hits found.
/// The third channel is the number of BVH node hits found if the ray also hit scene geometry.
pub struct BVHIntersections {}

impl Integrator for BVHIntersections {
    fn li(
        &self,
        _scratch: &ScopedScratch,
        ray: Ray<f32>,
        scene: &Scene,
        _depth: u32,
        _sampler: &mut Box<dyn Sampler>,
    ) -> RadianceResult {
        let IntersectionResult {
            hit,
            intersection_test_count,
            intersection_count,
        } = scene.bvh.intersect(ray);
        let ray_count = 1;

        let color = Spectrum::new(
            intersection_test_count as f32,
            intersection_count as f32,
            if hit.is_some() {
                intersection_count as f32
            } else {
                0.0
            },
        );

        RadianceResult {
            li: color,
            ray_scene_intersections: ray_count,
        }
    }
}
