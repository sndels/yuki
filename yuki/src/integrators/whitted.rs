use super::base::{IntegratorBase, RadianceResult};
use crate::{
    bvh::IntersectionResult,
    materials::BxdfType,
    math::{Ray, Vec3},
    scene::Scene,
    shapes::Hit,
};

pub struct WhittedIntegrator;

impl IntegratorBase for WhittedIntegrator {
    fn li(ray: Ray<f32>, scene: &Scene) -> RadianceResult {
        let IntersectionResult { hit, .. } = scene.bvh.intersect(ray);
        let ray_count = 1;

        let incoming_radiance = if let Some(Hit { si, .. }) = hit {
            // TODO: Do color/spectrum class for this math
            fn mul(v1: Vec3<f32>, v2: Vec3<f32>) -> Vec3<f32> {
                Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
            }
            scene.lights.iter().fold(Vec3::from(0.0), |c, l| {
                let light_sample = l.sample_li(&si);
                // TODO: Trace light visibility
                c + mul(
                    si.bsdf
                        .as_ref()
                        .unwrap()
                        .f(si.wo, light_sample.l, BxdfType::all()),
                    light_sample.li,
                ) * si.n.dot_v(light_sample.l).clamp(0.0, 1.0)
            })
        } else {
            scene.background
        };

        RadianceResult {
            li: incoming_radiance,
            ray_scene_intersections: ray_count,
        }
    }
}
