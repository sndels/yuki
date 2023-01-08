use super::{Integrator, RadianceResult};
use crate::{
    bvh::IntersectionResult,
    interaction::{ShadingGeometry, SurfaceInteraction},
    math::{Ray, Spectrum},
    sampling::Sampler,
    scene::Scene,
    shapes::Hit,
};

use allocators::ScopedScratch;

pub struct ShadingNormals {}

impl Integrator for ShadingNormals {
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
                si:
                    SurfaceInteraction {
                        shading: ShadingGeometry { n, .. },
                        ..
                    },
                ..
            }) => Spectrum::new(n.x, n.y, n.z) / 2.0 + 0.5,
            None => Spectrum::zeros(),
        };

        RadianceResult {
            li: color,
            ray_scene_intersections: ray_count,
        }
    }
}
