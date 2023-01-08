use super::{Integrator, RadianceResult};
use crate::{
    bvh::IntersectionResult,
    interaction::SurfaceInteraction,
    math::{Ray, Spectrum},
    sampling::Sampler,
    scene::Scene,
    shapes::Hit,
};

use allocators::ScopedScratch;

pub struct ShadingUVs {}

impl Integrator for ShadingUVs {
    fn li(
        &self,
        _scratch: &ScopedScratch,
        ray: Ray<f32>,
        scene: &Scene,
        _depth: u32,
        _sampler: &mut Box<dyn Sampler>,
    ) -> RadianceResult {
        let IntersectionResult { hit, .. } = scene.bvh.intersect(ray);
        let ray_count = 1;

        let color = match hit {
            Some(Hit {
                si: SurfaceInteraction { uv, .. },
                ..
            }) => Spectrum::new(uv.x, uv.y, 0.0),
            None => Spectrum::zeros(),
        };

        RadianceResult {
            li: color,
            ray_scene_intersections: ray_count,
            ..RadianceResult::default()
        }
    }
}
