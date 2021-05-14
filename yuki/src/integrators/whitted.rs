use super::Integrator;
use crate::{
    math::{Ray, Vec3},
    scene::Scene,
    shapes::Hit,
};

pub struct WhittedIntegrator;

impl Integrator for WhittedIntegrator {
    fn li(ray: Ray<f32>, scene: &Scene) -> (Vec3<f32>, usize) {
        let hit = scene.bvh.intersect(ray);
        let ray_count = 1;

        let incoming_radiance = if let Some(Hit { si, .. }) = hit {
            // TODO: Do color/spectrum class for this math
            fn mul(v1: Vec3<f32>, v2: Vec3<f32>) -> Vec3<f32> {
                Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
            }
            scene.lights.iter().fold(Vec3::from(0.0), |c, l| {
                let light_sample = l.sample_li(&si);
                // TODO: Trace light visibility
                c + mul(si.albedo / std::f32::consts::PI, light_sample.li)
                    * si.n.dot_v(light_sample.l).clamp(0.0, 1.0)
            })
        } else {
            scene.background
        };

        (incoming_radiance, ray_count)
    }
}
