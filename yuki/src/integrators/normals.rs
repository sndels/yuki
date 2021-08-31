use super::{Integrator, RadianceResult};
use crate::{
    bvh::IntersectionResult,
    interaction::SurfaceInteraction,
    math::{Ray, Spectrum},
    sampling::Sampler,
    scene::Scene,
    shapes::Hit,
};

/// The first channel of returned color is the number of BVH intersections performed.
/// The second channel is the number of BVH node hits found.
/// The third channel is the number of BVH node hits found if the ray also hit scene geometry.
pub struct Normals {}

impl Integrator for Normals {
    fn li(
        &self,
        ray: Ray<f32>,
        scene: &Scene,
        _depth: u32,
        _sampler: &mut Box<dyn Sampler>,
        _collect_rays: bool,
    ) -> RadianceResult {
        let IntersectionResult { hit, .. } = scene.bvh.intersect(ray);
        let ray_count = 1;

        let color = match hit {
            Some(Hit {
                si: SurfaceInteraction { n, .. },
                ..
            }) => Spectrum::new(n.x, n.y, n.z) / 2.0 + 0.5,
            None => Spectrum::zeros(),
        };

        RadianceResult {
            li: color,
            ray_scene_intersections: ray_count,
            ..RadianceResult::default()
        }
    }
}
